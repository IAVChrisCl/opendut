use core::panic;
use std::time::Instant;
use std::collections::HashMap;

use autosar_data::{AutosarModel, CharacterData, Element, ElementName, EnumItem};

/*
- Arxml parser that is able to extract all values necessary for a restbus simulation
- See main method for usage example.
*/

/* 
- TODO: 
    - finish parsing and fill up structures 
    - What about TPConfig and get_init_value_from_signals and get_init_value_from_signals?
    - create restbus simulation based on parsed data in a different source code file

- Improvements at some stage:
    - Provide options to store parsed data for quicker restart
    - Put structure defintions in separete source code file
    - be able to manually add stuff to restbus -> provide interface

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
    rx_range_lower: i64,
    rx_range_upper: i64,
    sender_ecus: Vec<String>,
    receiver_ecus: Vec<String>,
    frame_length: i64,
    pdu_mappings: Vec<PDUMapping>
}

pub struct PDUMapping {
    name: String,
    byte_order: String,
    start_position: i64,
    length: i64,
    dynamic_length: String,
    category: String,
    contained_header_id_short: String,
    contained_header_id_long: String,
    pdu: PDU
}

enum PDU {
    ISignalIPDU(ISignalIPDU),
//     DCMIPDU(DCMIPDU),
//    NMPDU(NMPDU),
//     ContaineredPDU(XY),
//    Temp(i64)
}

/*pub struct DCMIPDU {  // Seems to be only DoIP relevant
    diag_pdu_type: String
}*/

/*pub struct NMPDU { // Seems to be only needed for Ethernet, not CAN
    nm_signal: String,
    start_pos: i64,
    length: i64
}*/

pub struct ISignalIPDU {
    cyclic_timing_period_value: f64,
    cyclic_timing_period_tolerance: Option<TimeRangeTolerance>,
    cyclic_timing_offset_value: f64,
    cyclic_timing_offset_tolerance: Option<TimeRangeTolerance>,
    number_of_repetitions: i64,
    repetition_period_value: f64,
    repetition_period_tolerance: Option<TimeRangeTolerance>,
    ungrouped_signals: Vec<ISignal>,
    grouped_signals: Vec<ISignalGroup>
}

pub struct ISignal {
    name: String,
    start_pos: i64,
    length: i64,
    init_value: i64
}

pub struct E2EDataTransformationProps {
    transformer_name: String,
    data_id: i64,
    data_length: i64
}

pub struct ISignalGroup {
    name: String,
    isignals: Vec<ISignal>,
    data_transformations: Vec<String>,
    transformation_props: Vec<E2EDataTransformationProps>
}

enum TimeRangeTolerance {
    Relative(i64),
    Absolute(f64),
}

struct TimeRange {
    tolerance: Option<TimeRangeTolerance>,
    value: f64,
}    
// Parser structure
pub struct ArxmlParser {
}

// Use autosar-data library to parse data like in this example:
// https://github.com/DanielT/autosar-data/blob/main/autosar-data/examples/businfo/main.rs
// Do I have to add license to this file or is project license enough?
impl ArxmlParser {
    /*
        HELPER METHODS START 
     */
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

    fn get_time_range(&self, base: &Element) -> Option<TimeRange> {
        let value = base
            .get_sub_element(ElementName::Value)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.double_value())?;
    
        let tolerance = if let Some(absolute_tolerance) = base
            .get_sub_element(ElementName::AbsoluteTolerance)
            .and_then(|elem| elem.get_sub_element(ElementName::Absolute))
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.double_value())
        {
            Some(TimeRangeTolerance::Absolute(absolute_tolerance))
        } else {
            base.get_sub_element(ElementName::RelativeTolerance)
                .and_then(|elem| elem.get_sub_element(ElementName::Relative))
                .and_then(|elem| elem.character_data())
                .and_then(|cdata| self.decode_integer(&cdata))
                .map(TimeRangeTolerance::Relative)
        };
    
        Some(TimeRange { tolerance, value })
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

    fn get_required_int_value(&self, element: &Element, subelement_name: ElementName) -> i64 {
        if let Some(int_value) = self.get_subelement_int_value(element, subelement_name) {
            return int_value;
        } else {
            panic!("Error getting required integer value of {}", subelement_name);
        }
    }

    fn get_optional_int_value(&self, element: &Element, subelement_name: ElementName) -> i64 {
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

    fn get_subelement_string_value(&self, element: &Element, subelement_name: ElementName) -> Option<String> {
        return element 
            .get_sub_element(subelement_name)
            .and_then(|elem| elem.character_data())
            .map(|cdata| cdata.to_string());
    }

    fn get_required_string(&self, element: &Element, subelement_name: ElementName) -> String {
        if let Some(value) = self.get_subelement_string_value(element, subelement_name) {
            return value;
        } else {
            panic!("Error getting required String value of {}", subelement_name);
        }
    }
    
    fn get_optional_string(&self, element: &Element, subelement_name: ElementName) -> String {
        if let Some(value) = self.get_subelement_string_value(element, subelement_name) {
            return value;
        } else {
            return String::from("");
        }
    }

    fn get_subelement_optional_string(&self, element: &Element, subelement_name: ElementName, sub_subelement_name: ElementName) -> String {
        if let Some(value) = element.get_sub_element(subelement_name)
            .and_then(|elem| elem.get_sub_element(sub_subelement_name))
            .and_then(|elem| elem.character_data())
            .map(|cdata| cdata.to_string()) 
        {
            return value;     
        } else {
            return String::from("");
        }
    }

    fn ecu_of_frame_port(&self, frame_port: &Element) -> Option<String> {
        let ecu_comm_port_instance = frame_port.parent().ok()??;
        let comm_connector = ecu_comm_port_instance.parent().ok()??;
        let connectors = comm_connector.parent().ok()??;
        let ecu_instance = connectors.parent().ok()??;
        ecu_instance.item_name()
    }
    /*
        HELPER METHODS END
     */

    fn handle_isignal_ipdu(&self, pdu: &Element) -> Option<ISignalIPDU> {
        // Find out these values: ...
        let mut cyclic_timing_period_value: f64 = 0_f64;
        let mut cyclic_timing_period_tolerance: Option<TimeRangeTolerance> = None; 

        let mut cyclic_timing_offset_value: f64 = 0_f64;
        let mut cyclic_timing_offset_tolerance: Option<TimeRangeTolerance> = None;
                
        let mut number_of_repetitions: i64 = 0;
        let mut repetition_period_value: f64 = 0_f64;
        let mut repetition_period_tolerance: Option<TimeRangeTolerance> = None;


        if let Some(tx_mode_true_timing) = pdu
            .get_sub_element(ElementName::IPduTimingSpecifications)
            .and_then(|elem| elem.get_sub_element(ElementName::IPduTiming))
            .and_then(|elem| elem.get_sub_element(ElementName::TransmissionModeDeclaration))
            .and_then(|elem| elem.get_sub_element(ElementName::TransmissionModeTrueTiming)) 
        {
            if let Some(cyclic_timing) = tx_mode_true_timing
                    .get_sub_element(ElementName::CyclicTiming) 
            {
                // Time period 
                if let Some(time_range) = cyclic_timing
                    .get_sub_element(ElementName::TimePeriod)
                    .and_then(|elem| self.get_time_range(&elem)) 
                {
                    cyclic_timing_period_value = time_range.value;
                    cyclic_timing_period_tolerance = time_range.tolerance;
                }

                // Time offset
                if let Some(time_range) = cyclic_timing
                    .get_sub_element(ElementName::TimeOffset)
                    .and_then(|elem| self.get_time_range(&elem)) 
                {
                    cyclic_timing_offset_value = time_range.value;
                    cyclic_timing_offset_tolerance = time_range.tolerance;
                }
            }
            if let Some(event_timing) = tx_mode_true_timing
                .get_sub_element(ElementName::EventControlledTiming) 
            {
                number_of_repetitions = self.get_optional_int_value(&event_timing, 
                    ElementName::NumberOfRepetitions);
                
                if let Some(time_range) = event_timing 
                    .get_sub_element(ElementName::RepetitionPeriod)
                    .and_then(|elem| self.get_time_range(&elem)) 
                {
                        repetition_period_value = time_range.value;
                        repetition_period_tolerance = time_range.tolerance;
                }
            }
        }

        //let mut signals: HashMap<String, (String, Option<i64>, Option<i64>)> = HashMap::new();
        let mut signals: HashMap<String, (String, i64, i64, i64)> = HashMap::new();
        let mut signal_groups = Vec::new();

        if let Some(isignal_to_pdu_mappings) = pdu.get_sub_element(ElementName::ISignalToPduMappings) {
            // collect information about the signals and signal groups
            for mapping in isignal_to_pdu_mappings.sub_elements() {
                if let Some(signal) = mapping
                    .get_sub_element(ElementName::ISignalRef)
                    .and_then(|elem| elem.get_reference_target().ok())
                {
                    let refpath = self.get_required_string(&mapping, 
                        ElementName::ISignalRef);

                    let name = self.get_required_item_name(&signal, "ISignalRef");

                    let start_pos = self.get_required_int_value(&mapping, 
                        ElementName::StartPosition);
                    
                    let length = self.get_required_int_value(&signal, 
                        ElementName::Length);

                    let mut init_value: i64 = 0;
                    if let Some(init_value_elem) = signal.get_sub_element(ElementName::InitValue) {
                        if let Some(num_val) = init_value_elem.get_sub_element(ElementName::NumericalValueSpecification) {
                            init_value = self.get_required_int_value(&num_val, ElementName::Value);
                        } else {
                            panic!("InitValue element does not have NumercialValueSpecification for signal {}", name);
                        }
                    } 
                    
                    signals.insert(refpath, (name, start_pos, length, init_value));
                } else if let Some(signal_group) = mapping
                    .get_sub_element(ElementName::ISignalGroupRef)
                    .and_then(|elem| elem.get_reference_target().ok())
                {
                    // store the signal group for now
                    signal_groups.push(signal_group);
                }
            }
        }
    
        let mut grouped_signals: Vec<ISignalGroup> = Vec::new();

        for signal_group in &signal_groups {
            let group_name = self.get_required_item_name(&signal_group, "ISignalGroupRef"); 
            
            let mut signal_group_signals: Vec<ISignal> = Vec::new();

            let isignal_refs = signal_group.get_sub_element(ElementName::ISignalRefs)?;

            // Removing ok and needed?
            for isignal_ref in isignal_refs.sub_elements()
                .filter(|elem| elem.element_name() == ElementName::ISignalRef) {
                if let Some(CharacterData::String(path)) = isignal_ref.character_data() {
                    if let Some(siginfo) = signals.get(&path) {
                        let siginfo_tmp = siginfo.clone();
                        let isginal_tmp: ISignal = ISignal {
                            name: siginfo_tmp.0,
                            start_pos: siginfo.1,
                            length: siginfo.2,
                            init_value: siginfo.3
                        };

                        signal_group_signals.push(isginal_tmp);
                        signals.remove(&path);
                    }
                }
            }

            let mut data_transformations: Vec<String> = Vec::new();

            if let Some(com_transformations) = signal_group
                .get_sub_element(ElementName::ComBasedSignalGroupTransformations) 
            {
                for elem in com_transformations.sub_elements() {
                    let data_transformation = self.get_required_reference(&elem,
                        ElementName::DataTransformationRef);
                    
                    data_transformations.push(self.get_required_item_name(
                            &data_transformation,
                            "DataTransformation"));
                }
            }

            let mut props_vector: Vec<E2EDataTransformationProps> = Vec::new();

            if let Some(transformation_props) = signal_group.get_sub_element(ElementName::TransformationISignalPropss) {
                for e2exf_props in transformation_props
                    .sub_elements()
                    .filter(|elem| elem.element_name() == ElementName::EndToEndTransformationISignalProps)
                {
                    if let Some(e2exf_props_cond) = e2exf_props
                        .get_sub_element(ElementName::EndToEndTransformationISignalPropsVariants)
                        .and_then(|elem| elem.get_sub_element(ElementName::EndToEndTransformationISignalPropsConditional))
                    {
                        let transformer_reference = self.get_required_reference(&e2exf_props_cond, 
                            ElementName::TransformerRef);
                        
                        let transformer_name = self.get_required_item_name(&transformer_reference, 
                            "TransformerName");

                        let data_ids = e2exf_props_cond
                            .get_sub_element(ElementName::DataIds)?;

                        let data_id = self.get_required_int_value(&data_ids,
                            ElementName::DataId);
                        
                        let data_length = self.get_required_int_value(&e2exf_props_cond,
                            ElementName::DataLength);
                        
                        
                        let props_struct: E2EDataTransformationProps = E2EDataTransformationProps {
                            transformer_name: transformer_name,
                            data_id: data_id,
                            data_length: data_length 
                        };

                        props_vector.push(props_struct);
                    }
                }
            }

            let isignal_group_struct: ISignalGroup = ISignalGroup {
                name: group_name,
                isignals: signal_group_signals,
                data_transformations: data_transformations,
                transformation_props: props_vector 
            };

            grouped_signals.push(isignal_group_struct);
        }

        // fill
        let mut ungrouped_signals: Vec<ISignal> = Vec::new();

        let remaining_signals: Vec<(String, i64, i64, i64)> = signals.values().cloned().collect();
        if remaining_signals.len() > 0 {
            for (name, start_pos, length, init_value) in remaining_signals {
                let isignal_struct: ISignal = ISignal {
                    name: name,
                    start_pos: start_pos,
                    length: length,
                    init_value: init_value
                };
                ungrouped_signals.push(isignal_struct);
            }
        }
        
        let isginal_ipdu: ISignalIPDU = ISignalIPDU {
            cyclic_timing_period_value: cyclic_timing_period_value,
            cyclic_timing_period_tolerance: cyclic_timing_period_tolerance,
            cyclic_timing_offset_value: cyclic_timing_offset_value,
            cyclic_timing_offset_tolerance: cyclic_timing_offset_tolerance,
            number_of_repetitions: number_of_repetitions,
            repetition_period_value: repetition_period_value,
            repetition_period_tolerance: repetition_period_tolerance,
            ungrouped_signals: ungrouped_signals, 
            grouped_signals: grouped_signals 
        };

        return Some(isginal_ipdu);
    }

    /*// Add support in future in case it is needed 
    fn handle_container_ipdu(&self, pdu: &Element){
        let mut container_timeout: f64 = 0.0;

        let header_type = self.get_optional_string(pdu, ElementName::HeaderType);

        if let Some(container_timeout_tmp) = pdu
            .get_sub_element(ElementName::ContainerTimeout)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.double_value())
        {
            container_timeout = container_timeout_tmp;
        }

        let container_trigger = self.get_optional_string(pdu, ElementName::ContainerTrigger);

        if let Some(contained_pdu_refs) = pdu.get_sub_element(ElementName::ContainedPduTriggeringRefs) {
            for contained_ref in contained_pdu_refs.sub_elements() {
                if let Some(contained_pdu) = contained_ref
                    .get_reference_target()
                    .ok()
                    .and_then(|elem| elem.get_sub_element(ElementName::IPduRef))
                    .and_then(|elem| elem.get_reference_target().ok())
                {
                    let pdu_name = self.get_required_item_name(&contained_pdu, "ContainedPDU");
                    display_pdu(&contained_pdu, indent + 1);
                }
            }
        }
        //...
    }*/

    /*// Add support in future in case it is needed 
    fn handle_secured_ipdu(&self, pdu: &Element){

    }*/

    fn handle_pdu_mapping(&self, pdu_mapping: &Element) -> Result<PDUMapping, String> {
        let pdu = self.get_required_reference(
            pdu_mapping,
            ElementName::PduRef);
        
        let pdu_name = self.get_required_item_name(
            &pdu, "Pdu");

        let byte_order = self.get_required_string(pdu_mapping, 
            ElementName::PackingByteOrder);

        let start_position = self.get_required_int_value(pdu_mapping, 
            ElementName::StartPosition);

        let pdu_length = self.get_required_int_value(&pdu, 
            ElementName::Length);
        
        let pdu_dynamic_length = self.get_optional_string(&pdu, 
            ElementName::HasDynamicLength);
        
        let pdu_category = self.get_optional_string(&pdu, 
            ElementName::Category);
        
        let pdu_contained_header_id_short = self.get_subelement_optional_string(&pdu, 
            ElementName::ContainedIPduProps, ElementName::HeaderIdShortHeader);
        
        let pdu_contained_header_id_long = self.get_subelement_optional_string(&pdu, 
            ElementName::ContainedIPduProps, ElementName::HeaderIdLongHeader);

        //let mut pdu_specific: PDU = PDU::Temp(0);
        let pdu_specific: PDU;

        match pdu.element_name() {
            ElementName::ISignalIPdu => {
                if let Some(value) = self.handle_isignal_ipdu(&pdu) {
                    pdu_specific = PDU::ISignalIPDU(value);
                } else {
                    panic!("Error in handle_isignal_ipdu");
                }
            }
            /*ElementName::ContainerIPdu => { // Add support in future if needed
                panic!("endounter containerpdu");
                //self.handle_container_ipdu(&pdu);
            }*/
            /*ElementName::SecuredIPdu => { // Add support in future if needed
                self.handle_secured_ipdu(&pdu);
            }*/
            // Handle more?
            _ => {
                let error = format!("PDU type {} not supported. Will skip it.", pdu.element_name().to_string());
                return Err(error)
            }
        }

        let pdu_mapping: PDUMapping = PDUMapping {
            name: pdu_name,
            byte_order: byte_order,
            start_position: start_position,
            length: pdu_length,
            dynamic_length: pdu_dynamic_length,
            category: pdu_category,
            contained_header_id_short: pdu_contained_header_id_short,
            contained_header_id_long: pdu_contained_header_id_long,
            pdu: pdu_specific 
        };

        return Ok(pdu_mapping);     
    }

    fn handle_can_frame_triggering(&self, can_frame_triggering: &Element) -> Result<CanFrameTriggering, String> {
        let can_frame_triggering_name= self.get_required_item_name(
            can_frame_triggering, "CanFrameTriggering");

        let can_id = self.get_required_int_value(
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

        let mut rx_range_lower: i64 = 0;
        let mut rx_range_upper: i64 = 0;
        if let Some(range_elem) = can_frame_triggering.get_sub_element(ElementName::RxIdentifierRange) {
            rx_range_lower = self.get_required_int_value(&range_elem, ElementName::LowerCanId);
            rx_range_upper = self.get_required_int_value(&range_elem, ElementName::UpperCanId);
        }

        let mut rx_ecus: Vec<String> = Vec::new();
        let mut tx_ecus: Vec<String> = Vec::new();
        if let Some(frame_ports) = can_frame_triggering.get_sub_element(ElementName::FramePortRefs) {
            let frame_ports: Vec<Element> = frame_ports.sub_elements()
                .filter(|se| se.element_name() == ElementName::FramePortRef)
                .filter_map(|fpr| fpr.get_reference_target().ok())
                .collect();

            for frame_port in frame_ports {
                if let Some(ecu_name) = self.ecu_of_frame_port(&frame_port) {
                    if let Some(CharacterData::Enum(direction)) = frame_port
                        .get_sub_element(ElementName::CommunicationDirection)
                        .and_then(|elem| elem.character_data())
                    {
                        match direction {
                            EnumItem::In => rx_ecus.push(ecu_name), 
                            EnumItem::Out => tx_ecus.push(ecu_name), 
                            _ => return Err(format!("Invalid direction ID encountered in FramePort. Skipping CanFrameTriggering {}", can_frame_triggering_name))
                        }
                    } else {
                        return Err(format!("No CommunicationDirection encountered in FramePort. Skipping CanFrameTriggering {}", can_frame_triggering_name)) 
                    }
                } else {
                    return Err(format!("Could not extract ECUName in FramePort. Skipping CanFrameTriggering {}", can_frame_triggering_name)) ;
                }
            }
        } else {
            return Err(format!("FramePortRefs in CanFrameTriggering not found. Skipping CanFrameTriggering {}", can_frame_triggering_name));
        }

        let frame_length = self.get_optional_int_value(
            &frame,
            ElementName::FrameLength);

        let mut pdu_mappings_vec: Vec<PDUMapping> = Vec::new();

        // assign here and other similar variable?
        if let Some(mappings) = frame.get_sub_element(ElementName::PduToFrameMappings) {
            for pdu_mapping in mappings.sub_elements() {
                match self.handle_pdu_mapping(&pdu_mapping) {
                    Ok(value) => pdu_mappings_vec.push(value),
                    Err(error) => return Err(error) 
                }
            }
        }

        let can_frame_triggering_struct: CanFrameTriggering = CanFrameTriggering {
            frame_triggering_name: can_frame_triggering_name,
            frame_name: frame_name,
            can_id: can_id,
            addressing_mode: addressing_mode,
            frame_rx_behavior: frame_rx_behavior,
            frame_tx_behavior: frame_tx_behavior,
            rx_range_lower: rx_range_lower,
            rx_range_upper: rx_range_upper,
            receiver_ecus: rx_ecus,
            sender_ecus: tx_ecus,
            frame_length: frame_length,
            pdu_mappings: pdu_mappings_vec 
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
        let can_cluster_baudrate =  self.get_optional_int_value(
            &can_cluster_conditional,
            ElementName::Baudrate);
        
        let can_cluster_fd_baudrate =  self.get_optional_int_value(
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
                        Err(error) => println!("[-] WARNING: {}", error),
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
        println!("Cluster: {}", cluster.name);
        println!("\tBaudrate: {}", cluster.baudrate);
        println!("\tFD Baudrate: {}", cluster.canfd_baudrate);
        for can_frame_triggering in cluster.can_frame_triggerings {
            println!("\tCanFrameTriggering: {}", can_frame_triggering.frame_triggering_name);
            println!("\t\tFrame Name: {}", can_frame_triggering.frame_name);
            println!("\t\tCAN ID: {}", can_frame_triggering.can_id);
            println!("\t\tAddressing Mode: {}", can_frame_triggering.addressing_mode);
            println!("\t\tFrame RX Behavior: {}", can_frame_triggering.frame_rx_behavior);
            println!("\t\tFrame TX Behavior: {}", can_frame_triggering.frame_tx_behavior);
            println!("\t\tRx Range Lower: {}", can_frame_triggering.rx_range_lower);
            println!("\t\tRx Range Upper: {}", can_frame_triggering.rx_range_upper);
            println!("\t\tSender ECUs:");
            for sender_ecu in can_frame_triggering.sender_ecus {
                println!("\t\t\tName: {}", sender_ecu);
            }
            println!("\t\tReceiver ECUs:");
            for receiver_ecu in can_frame_triggering.receiver_ecus {
                println!("\t\t\tName: {}", receiver_ecu);
            }
            println!("\t\tFrame Length: {}", can_frame_triggering.frame_length);
            for pdu_mapping in can_frame_triggering.pdu_mappings {
                println!("\t\tPDUMapping: {}", pdu_mapping.name);
                println!("\t\t\tByte Order: {}", pdu_mapping.byte_order);
                println!("\t\t\tStart Position: {}", pdu_mapping.start_position);
                println!("\t\t\tLength: {}", pdu_mapping.length);
                println!("\t\t\tDynamic Length: {}", pdu_mapping.dynamic_length);
                println!("\t\t\tCategory: {}", pdu_mapping.category);
                println!("\t\t\tContained Header ID Short: {}", pdu_mapping.contained_header_id_short);
                println!("\t\t\tContained Header ID Long: {}", pdu_mapping.contained_header_id_long);
                match pdu_mapping.pdu {
                    PDU::ISignalIPDU(pdu) => {
                        println!("\t\t\tISignalPDU =>");
                        println!("\t\t\t\tCyclic Timing in s: {}", pdu.cyclic_timing_period_value);
                        if let Some(cyclic_timing_period_tolerance) = pdu.cyclic_timing_period_tolerance {
                            match cyclic_timing_period_tolerance {
                                TimeRangeTolerance::Absolute(value) => println!("\t\t\t\tCyclic Timing Tolerance in s: {}", value),
                                TimeRangeTolerance::Relative(value) => println!("\t\t\t\tCyclic Timing Tolerance in %: {}", value),
                            }
                        }
                        println!("\t\t\t\tCyclic Timing Offset in s: {}", pdu.cyclic_timing_offset_value);
                        if let Some(cyclic_timing_offset_tolerance) = pdu.cyclic_timing_offset_tolerance {
                            match cyclic_timing_offset_tolerance {
                                TimeRangeTolerance::Absolute(value) => println!("\t\t\t\tCyclic Timing Offset Tolerance in s: {}", value),
                                TimeRangeTolerance::Relative(value) => println!("\t\t\t\tCyclic Timing Offset Tolerance in %: {}", value),
                            }
                        }
                        println!("\t\t\t\tNumber of Repetitions: {}", pdu.number_of_repetitions);
                        println!("\t\t\t\tRepetition Period: {}", pdu.repetition_period_value);
                        if let Some(repetition_period_tolerance) = pdu.repetition_period_tolerance {
                            match repetition_period_tolerance {
                                TimeRangeTolerance::Absolute(value) => println!("\t\t\t\tRepetition Period Tolerance in s: {}", value),
                                TimeRangeTolerance::Relative(value) => println!("\t\t\t\tRepetition Period Tolerance in %: {}", value),
                            }
                        }
                        for isignal_group in pdu.grouped_signals {
                            println!("\t\t\t\tISignal Group: {}", isignal_group.name);
                            for isignal in isignal_group.isignals {
                                println!("\t\t\t\t\tISignal: {}", isignal.name);
                                println!("\t\t\t\t\t\tStart Position: {}", isignal.start_pos);
                                println!("\t\t\t\t\t\tLength: {}", isignal.length);
                                println!("\t\t\t\t\t\tInit Value: {}", isignal.init_value);
                            }
                            for data_transformation in isignal_group.data_transformations {
                                println!("\t\t\t\t\tData Transformation: {}", data_transformation);
                            }
                            for transformation_prop in isignal_group.transformation_props {
                                println!("\t\t\t\t\tE2E Transformer Properties: {}", transformation_prop.transformer_name);
                                println!("\t\t\t\t\t\tData ID: {}", transformation_prop.data_id);
                                println!("\t\t\t\t\t\tData Length: {}", transformation_prop.data_length);
                                panic!("ok");
                            }
                        }
                        for isignal in pdu.ungrouped_signals {
                                println!("\t\t\t\tUngrouped ISignal: {}", isignal.name);
                                println!("\t\t\t\t\tStart Position: {}", isignal.start_pos);
                                println!("\t\t\t\t\tLength: {}", isignal.length);
                                println!("\t\t\t\t\tInit Value: {}", isignal.init_value);
                        }
                    }
                    //_ => {}
                }
            }
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

