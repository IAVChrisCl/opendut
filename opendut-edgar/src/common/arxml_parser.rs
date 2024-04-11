use core::panic;
use std::time::Instant;

use autosar_data::{AutosarModel, CharacterData, Element, ElementName, EnumItem};

/*
- Arxml parser that is able to extract all values necessary for a restbus simulation
- See main method for usage example.
*/

/* 
- TODO: 
    - finish parsing
    - move strucutures to separate file and import them
    - resolve references
    - create restbus simulation based on parsed data in a differenc source code file

- Improvements at some stage:
    - be able to manually add stuff to restbus -> provide interface
    - Use hash maps for quicker reference resolving
    - Use hash maps or precalulcated hashes for XML element flag setting
    - increase parsing speed by skipping ar-packages not of interest -> No success 
    - support multiple can-cluster variants and physical channels
    - Currently using xml-rs. Use quick_xml when applicable

- Code inside DEBUG comments will be removed at a later stage
*/


// Future restbus simulation structure used to setup and control restbus simulation. Will be moved to seprarate source code file.
pub struct RestbusSimulation {

}

pub struct CanCluster {
    name: String,
    baudrate: i64,
    canfd_baudrate: i64,
    can_frame_triggerings: Vec<CanFrameTriggering>
}
pub struct CanFrameTriggering {
    frame_triggering_name: String,
    frame_name: String,
    can_id: i64,
    addressing_mode: String,
    frame_rx_behavior: String,
    frame_tx_behavior: String,
    frame_length: i64,
    pdu_mappings: Vec<PDUMapping>
}

pub struct PDUMapping {
}

// Parser structure
pub struct ArxmlParser {
}

// Use autosar-data library to parse data like in this example:
// https://github.com/DanielT/autosar-data/blob/main/autosar-data/examples/businfo/main.rs
// Do I have to add license to this file or is project license enough?
impl ArxmlParser {
    fn decode_integer(&self, cdata: &CharacterData) -> Option<i64> {
        if let CharacterData::String(text) = cdata {
            if text == "0" {
                Some(0)
            } else if text.starts_with("0x") {
                let hexstr = text.strip_prefix("0x").unwrap();
                Some(i64::from_str_radix(hexstr, 16).ok()?)
            } else if text.starts_with("0X") {
                let hexstr = text.strip_prefix("0X").unwrap();
                Some(i64::from_str_radix(hexstr, 16).ok()?)
            } else if text.starts_with("0b") {
                let binstr = text.strip_prefix("0b").unwrap();
                Some(i64::from_str_radix(binstr, 2).ok()?)
            } else if text.starts_with("0B") {
                let binstr = text.strip_prefix("0B").unwrap();
                Some(i64::from_str_radix(binstr, 2).ok()?)
            } else if text.starts_with('0') {
                let octstr = text.strip_prefix('0').unwrap();
                Some(i64::from_str_radix(octstr, 8).ok()?)
            } else {
                Some(text.parse().ok()?)
            }
        } else {
            None
        }
    }

    fn get_required_item_name(&self, element: &Element, element_name: &str) -> String {
        if let Some(item_name) = element.item_name() {
            return item_name; 
        } else {
            panic!("Error getting required item name of {}", element_name);
        } 
    }

    fn get_required_sub_subelement(&self, element: &Element, subelement_name: ElementName, sub_subelement_name: ElementName) -> Element {
        if let Some(sub_subelement) = element 
            .get_sub_element(subelement_name)
            .and_then(|elem| elem.get_sub_element(sub_subelement_name)) 
        {
            return sub_subelement;
        } else {
            panic!("Error getting sub_subelement. Tried to retrieve {} and then {}",
                subelement_name,
                sub_subelement_name);
        } 
    }

    fn get_subelement_int_value(&self, element: &Element, subelement_name: ElementName) -> Option<i64> {
        return element 
            .get_sub_element(subelement_name)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| self.decode_integer(&cdata));
    } 

    fn get_required_subelement_int_value(&self, element: &Element, subelement_name: ElementName) -> i64 {
        if let Some(int_value) = self.get_subelement_int_value(element, subelement_name) {
            return int_value;
        } else {
            panic!("Error getting required integer value of {}", subelement_name);
        }
    }

    fn get_optional_subelement_int_value(&self, element: &Element, subelement_name: ElementName) -> i64 {
        if let Some(int_value) = self.get_subelement_int_value(element, subelement_name) {
            return int_value;
        } else {
            return 0;
        }
    }

    fn get_required_reference(&self, element: &Element, subelement_name: ElementName) -> Element {
        if let Some(subelement) = element.get_sub_element(subelement_name) {
            match subelement.get_reference_target() {
                Ok(reference) => return reference,
                Err(_) => {} 
            }
        }
        
        panic!("Error getting required reference for {}", subelement_name);
    }

    fn get_optional_string(&self, element: &Element, subelement_name: ElementName) -> String {
        if let Some(value) = element 
            .get_sub_element(subelement_name)
            .and_then(|elem| elem.character_data())
            .map(|cdata| cdata.to_string()) 
        {
            return value;
        } else {
            return String::from("");
        }
    }

    /*fn handle_isignal_ipdu(&self, pdu: &Element){
        // Find out these values: ...

        if let Some(tx_mode_true_timing) = pdu
            .get_sub_element(ElementName::IPduTimingSpecifications)
            .and_then(|elem| elem.get_sub_element(ElementName::IPduTiming))
            .and_then(|elem| elem.get_sub_element(ElementName::TransmissionModeDeclaration))
            .and_then(|elem| elem.get_sub_element(ElementName::TransmissionModeTrueTiming))
        {
            if let Some(cyclic_timing) = tx_mode_true_timing
                .get_sub_element(ElementName::CyclicTiming) 
            {
                if let Some(TimeRange { tolerance, value }) = cyclic_timing
                    .get_sub_element(ElementName::TimePeriod)
                    .and_then(|elem| get_time_range(&elem))
                {
                    let cyclic_timing_value = value;
                    match tolerance {
                        Some(TimeRangeTolerance::Absolute(absval)) => {
                            let cyclic_timing_tolerance_absolute = absval; // in seconds
                        }
                        Some(TimeRangeTolerance::Relative(relval)) => {
                            let cyclic_timing_tolerance_relative = relval; // in %
                        }
                        _ => {}
                    }

                    if let Some(TimeRange { tolerance, value }) = cyclic_timing
                        .get_sub_element(ElementName::TimeOffset)
                        .and_then(|elem| get_time_range(&elem))
                    {
                        let cyclic_timing_offset_value = value;
                        match tolerance {
                            Some(TimeRangeTolerance::Absolute(absval)) => {
                                let cyclic_timing_offset_tolerance_absolute = absval; // in seconds
                            }
                            Some(TimeRangeTolerance::Relative(relval)) => {
                                let cyclic_timing_offset_tolerance_relative = relval; // in seconds
                            }
                            _ => {}
                        }
                    }
                }
            }

            if let Some(event_timing) = tx_mode_true_timing.get_sub_element(ElementName::EventControlledTiming) {
                if let Some(num_reps) = event_timing
                    .get_sub_element(ElementName::NumberOfRepetitions)
                    .and_then(|elem| elem.character_data())
                    .and_then(|cdata| decode_integer(&cdata))
                {
                    let number_of_repetitions = num_reps;
                }
                if let Some(repetition_period) = event_timing.get_sub_element(ElementName::RepetitionPeriod) {
                    if let Some(TimeRange { tolerance, value }) = get_time_range(&repetition_period) {
                        let repetition_period = value;
                        if let Some(tol) = tolerance {
                            match tol {
                                TimeRangeTolerance::Relative(percent) => {
                                    let repetition_period_tolerance_relative = percent;
                                }
                                TimeRangeTolerance::Absolute(abstol) => {
                                    let repetition_period_tolerance_absoulte = abstol;
                                }
                            }
                        }
                    }
                }
            }

            // Continue and handle signals
        }
    }

    fn handle_dcm_ipdu(&self, pdu: &Element){
        
    }
    
    fn handle_nm_pdu(&self, pdu: &Element){

    }
    
    fn handle_container_ipdu(&self, pdu: &Element){

    }
    
    fn handle_secured_ipdu(&self, pdu: &Element){

    }*/

    /*fn handle_pdu_mapping(&self, pdu_mapping: &Element) -> Option<()> {
        let pdu = pdu_mapping
            .get_sub_element(ElementName::PduRef)
            .and_then(|pduref| pduref.get_reference_target().ok())?;

        let pdu_name = pdu.item_name()?; 

        let byte_order = pdu_mapping
            .get_sub_element(ElementName::PackingByteOrder)
            .and_then(|elem| elem.character_data())
            .map(|cdata| cdata.to_string());

        let start_position = pdu_mapping
            .get_sub_element(ElementName::StartPosition)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.unsigned_integer_value());

        let pdu_length = pdu
            .get_sub_element(ElementName::Length)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.unsigned_integer_value());

        let pdu_dynamic_length = pdu
            .get_sub_element(ElementName::HasDynamicLength)
            .and_then(|elem| elem.character_data());

        match pdu.element_name() {
            ElementName::ISignalIPdu => {
                self.handle_isignal_ipdu(&pdu);
            }
            ElementName::DcmIPdu => {
                self.handle_dcm_ipdu(&pdu);
            }
            ElementName::NmPdu => {
                self.handle_nm_pdu(&pdu);
            }
            ElementName::GeneralPurposeIPdu => {}
            ElementName::NPdu => {}
            ElementName::XcpPdu => {}
            ElementName::ContainerIPdu => {
                self.handle_container_ipdu(&pdu);
            }
            ElementName::SecuredIPdu => {
                self.handle_secured_ipdu(&pdu);
            }
            ElementName::GeneralPurposePdu => {}
            _ => {
                panic!("PDU type not supported.")
            }
        }

        Some(())
    }*/

    fn handle_can_frame_triggering(&self, can_frame_triggering: &Element) -> Result<CanFrameTriggering, String> {
        let can_frame_triggering_name= self.get_required_item_name(
            can_frame_triggering, "CanFrameTriggering");

        let can_id = self.get_required_subelement_int_value(
            &can_frame_triggering,
            ElementName::Identifier);

        let frame = self.get_required_reference(
            can_frame_triggering,
            ElementName::FrameRef);

        let frame_name = self.get_required_item_name(
            &frame, "Frame");

        let addressing_mode = if let Some(CharacterData::Enum(value)) = can_frame_triggering
            .get_sub_element(ElementName::CanAddressingMode)
            .and_then(|elem| elem.character_data()) 
        {
            value.to_string()
        } else {
            EnumItem::Standard.to_string()
        };

        let frame_rx_behavior = self.get_optional_string(
            can_frame_triggering,
            ElementName::CanFrameRxBehavior);
        
        let frame_tx_behavior = self.get_optional_string(
            can_frame_triggering,
            ElementName::CanFrameTxBehavior);

        let frame_length = self.get_optional_subelement_int_value(
            &frame,
            ElementName::FrameLength);

        // assign here and other similar variable?
        /*if let Some(mappings) = frame.get_sub_element(ElementName::PduToFrameMappings) {
            for pdu_mapping in mappings.sub_elements() {
                self.handle_pdu_mapping(&pdu_mapping);
            }
        }*/ 

        let can_frame_triggering_struct: CanFrameTriggering = CanFrameTriggering {
            frame_triggering_name: can_frame_triggering_name,
            frame_name: frame_name,
            can_id: can_id,
            addressing_mode: addressing_mode,
            frame_rx_behavior: frame_rx_behavior,
            frame_tx_behavior: frame_tx_behavior,
            frame_length: frame_length,
            pdu_mappings: Vec::new() 
        };
 
        return Ok(can_frame_triggering_struct);
    }

    fn handle_can_cluster(&self, can_cluster: &Element) -> Result<CanCluster, String> {
        let can_cluster_name = self.get_required_item_name(
            can_cluster, "CanCluster");

        let can_cluster_conditional = self.get_required_sub_subelement(
            can_cluster, 
            ElementName::CanClusterVariants,
            ElementName::CanClusterConditional);

        //let can_cluster_baudrate =  self.get_required_subelement_int_value(
        let can_cluster_baudrate =  self.get_optional_subelement_int_value(
            &can_cluster_conditional,
            ElementName::Baudrate);
        
        let can_cluster_fd_baudrate =  self.get_optional_subelement_int_value(
            &can_cluster_conditional,
            ElementName::CanFdBaudrate);

        if can_cluster_baudrate == 0 && can_cluster_fd_baudrate == 0 {
            let msg = format!("Baudrate and FD Baudrate of CanCluster {} do not exist or are 0. Skipping this CanCluster.", can_cluster_name);
            return Err(msg.to_string());
        }

        // iterate over PhysicalChannels and handle the CanFrameTriggerings inside them
        let physical_channels;
        if let Some(value) = can_cluster_conditional
            .get_sub_element(ElementName::PhysicalChannels).map(|elem| {
                elem.sub_elements().filter(|se| se.element_name() == ElementName::CanPhysicalChannel)
            }) 
        {
            physical_channels = value;
        } else {
            let msg = format!("Cannot handle physical channels of CanCluster {}", can_cluster_name);
            return Err(msg.to_string());
        }

        let mut can_frame_triggerings: Vec<CanFrameTriggering> = Vec::new(); 
        for physical_channel in physical_channels {
            if let Some(frame_triggerings) = physical_channel.get_sub_element(ElementName::FrameTriggerings) {
                for can_frame_triggering in frame_triggerings.sub_elements() {
                    match self.handle_can_frame_triggering(&can_frame_triggering) {
                        Ok(value) => can_frame_triggerings.push(value),
                        Err(error) => return Err(error)
                    }
                }
            }
        }

        let can_cluster_struct: CanCluster = CanCluster {
            name: can_cluster_name,
            baudrate: can_cluster_baudrate,
            canfd_baudrate: can_cluster_fd_baudrate,
            can_frame_triggerings: can_frame_triggerings
        };
        
        return Ok(can_cluster_struct);
    }

    // Main parsing method. Uses autosar-data libray for parsing ARXML 
    // In the future, it might be extended to support Etherneth, Flexray, ...
    // Returns now a vector of CanCluster
    pub fn parse_file(&self, file_name: String) -> Option<Vec<CanCluster>> {
        let start = Instant::now();

        let model = AutosarModel::new();

        if let Err(err) = model.load_file(file_name, false) {
            panic!("Parsing failed. Error: {}", err.to_string());
        }

        // DEBUG 
        println!("[+] Duration of loading was: {:?}", start.elapsed());
        // DEBUG END

        let mut can_clusters: Vec<CanCluster> = Vec::new();

        // Iterate over Autosar elements and handle CanCluster elements
        for element in model
            .identifiable_elements()
            .iter()
            .filter_map(|path| model.get_element_by_path(&path))
        {
            match element.element_name() {
                ElementName::CanCluster => {
                    let result = self.handle_can_cluster(&element);
                    match result {
                        Ok(value) => can_clusters.push(value),
                        Err(error) => println!("[-] WARNING: {}", error)
                    }
                }
                _ => {}
            }
        }

        println!("[+] Duration of parsing: {:?}", start.elapsed());

        return Some(can_clusters);
    }
}

// Debug output. Code can be later reused with modificaitons in Restbus Simulaiton setup
fn test_data(can_clusters: Vec<CanCluster>) -> bool {
    for cluster in can_clusters {
        println!("Got CAN Cluster:");
        println!("\tCluster name: {}", cluster.name);
        println!("\tBaudrate: {}", cluster.baudrate);
        println!("\tFD Baudrate: {}", cluster.canfd_baudrate);
        for can_frame_triggering in cluster.can_frame_triggerings {
            println!("\tGot CanFrameTriggering: {}", can_frame_triggering.frame_triggering_name);
            println!("\t\tFrame name: {}", can_frame_triggering.frame_name);
            println!("\t\tCAN ID: {}", can_frame_triggering.can_id);
            println!("\t\tAddressing mode: {}", can_frame_triggering.addressing_mode);
            println!("\t\tFrame RX behavior: {}", can_frame_triggering.frame_rx_behavior);
            println!("\t\tFrame TX behavior: {}", can_frame_triggering.frame_tx_behavior);
            println!("\t\tFrame length: {}", can_frame_triggering.frame_length);
        }
    }

    return true;
}

fn main() {
    println!("[+] Starting openDuT ARXML parser over main method.");
    
    let file_name = "test.arxml";

    let arxml_parser: ArxmlParser = ArxmlParser {};

    if let Some(can_clusters) = arxml_parser
        .parse_file(file_name.to_string()) 
    {
        test_data(can_clusters);
    } else {
        panic!("Parsing failed.")
    }
}

