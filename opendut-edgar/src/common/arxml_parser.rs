use std::fs::File;
use std::io::BufReader;
use std::collections::HashSet;
use std::time::Instant;

use xml::reader::{EventReader, XmlEvent};

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

// Can-Frame-Triggering element inside a Can-Cluster element of a Cluster package
// Some values will be skipped when parsing. These will be filled correctly when resolving references
pub struct CanFrameTriggering {
    frame_ref: String,
    identifier: String,
    is_canfd: bool,
    //dlc: i8,
    //...
}

// Can-Cluster structure representing a Can-Cluster element inside a Cluster package
pub struct CanCluster {
    name: String,
    baudrate: i32,
    canfd_baudrate: i32,
    sum_physical_channels: i32,
    can_frame_triggerings: Vec<CanFrameTriggering>
    // config?
}

// Can-Frame element inside the Frame package
pub struct CanFrame {
    frame_length: i8,
    pdu_ref: String
}

// Pdu elements inside the Pdu package
pub struct Pdu {
    pdu_type: String,
    name: String,
    cyclic: f32,
    data: Vec<u8>,
    data_length: i8,
    crc_offset: i8,
    counter_offset: i8
}

// Parser structure
pub struct ArxmlParser {
}

impl ArxmlParser {
    // Create BufReader for a local file
    pub fn create_reader(&self, file_name: &str) -> BufReader<File> {
        println!("[+] Called ArxmlParser.create_reader");

        let file = match File::open(file_name) {
            Err(_) => panic!("Could not open file"),
            Ok(file) => file,
        };

        return BufReader::new(file)
    }

    // DEBUG
    // Check depth of encountered XML element. Can be removed at a later stage
    pub fn depth_check(&self, depth: i32, depth_expected: i32, name: &str, opening: bool) {
        if depth != depth_expected && 1 == 2 { // watch out
            if opening {
                panic!("Error at package depth check for opening {}. Depth is {} but should be {}", name, depth, depth_expected);
            } else {
                panic!("Error at package depth check for closing {}. Depth is {} but should be {}", name, depth, depth_expected);
            }
        }
    }
    // DEBUG END

    // Main parsing method. Requires a BufReader instance as argument. Parses Arxml structure and extract all values necessary for restbus simulation. 
    pub fn parse_data(&self, xml_reader: BufReader<File>) -> bool {
        println!("[+] Called ArxmlParser.parse_data");
        println!("[+] Parsing Arxml from BufReader instance");

        let start = Instant::now();

        let mut can_clusters: Vec<CanCluster> = Vec::new();

        let parser = EventReader::new(xml_reader);

        // DEBUG
        let mut count = 0;
        // DEBUG END

        let mut depth = 0; // 1 = autosar, 2 = ar-packages, 3 = ar-package/package

        let mut targeted_packages: HashSet<String> = HashSet::new();
        targeted_packages.insert("cluster".to_string());
        targeted_packages.insert("frame".to_string());
        targeted_packages.insert("pdu".to_string());

        // FLAGS used for mainly detecting if we are inside of XML elements
        // Usee hash set instead?
        let mut skip_package: bool = false;
        let mut inside_short_name: bool= false;
        let mut inside_cluster: bool = false;
        let mut inside_frame: bool= false;
        let mut inside_pdu: bool= false;
        let mut inside_can_cluster: bool = false;
        let mut inside_baudrate: bool = false;
        let mut inside_canfd_baudrate: bool = false;
        let mut inside_can_frame_triggering: bool = false;
        let mut inside_identifier: bool = false;
        let mut inside_can_frame_tx_behavior: bool = false;
        let mut inside_frame_ref: bool = false;
        let mut inside_can_frame: bool = false;
        let mut inside_frame_length: bool = false;
        let mut inside_pdu_ref: bool = false;
        let mut inside_pdu_element: bool = false;
        let mut inside_cyclic_timing: bool = false;
        let mut inside_time_period: bool = false;
        let mut inside_value: bool = false;

        // Temporary values to store CAN cluster data
        let mut can_cluster_name: String = String::from(""); 
        let mut can_cluster_baudrate: i32 = 0;
        let mut can_cluster_canfd_baudrate: i32 = 0;
        let mut can_cluster_sum_physical_channels: i32 = 0;
        let mut can_frame_triggerings: Vec<CanFrameTriggering> = Vec::new();

        // Temporary values to store CAN-Frame-Triggering data
        let mut cft_frame_ref: String = String::from("");
        let mut cft_identifier: String = String::from("");
        let mut cft_is_canfd: bool = false;

        // Temporary values to stare Can-Frame data
        let mut frame_length: i8 = 0; 
        let mut pdu_ref: String = String::from(""); 

        // Store Can-Frame elements data
        let mut can_frames: Vec<CanFrame> = Vec::new(); 
        
        // Temporary values for PDU element data
        let mut pdu_type: String = String::from(""); 
        let mut pdu_name: String = String::from("");
        let mut pdu_cyclic: f32 = 0.0;
        let mut pdu_data: Vec<u8> = Vec::new();
        let mut pdu_data_length: i8 = 0;
        let mut pdu_crc_offset: i8 = 0;
        let mut pdu_counter_offset: i8 = 0;

        let mut pdus: Vec<Pdu> = Vec::new();

        // Iterate over XML using XMLEvents of xml-rs. Extract important values and references. Use these to fill structures. References will be resolved after this loop.
        for event in parser {
            // DEBUG
            count += 1;
            if count > 10000 && 2 == 1 {
                println!("Done");
                break;
            }
            // DEBUG END

            match event {
                Err(error) => {
                    panic!("Error parsing XML event: {}", error);
                }

                Ok(XmlEvent::StartElement { name, .. }) => {
                    depth += 1;

                    match name.local_name.to_lowercase().as_str() {
                        "autosar" => self.depth_check(depth, 1, "<autosar>", true),
                        "ar-packages" => self.depth_check(depth, 2, "<ar-packages>", true),
                        "ar-package" | "package" => self.depth_check(depth, 3, "<ar-package> or <package>", true),
                        "can-cluster" => {
                            self.depth_check(depth, 5, "<can-cluster>", true);
                            if inside_cluster {
                                inside_can_cluster = true;

                                can_cluster_name = String::from(""); 
                                can_cluster_baudrate = 0;
                                can_cluster_canfd_baudrate = 0;
                                can_cluster_sum_physical_channels = 0;
                                can_frame_triggerings = Vec::new();
                            }
                        }
                        "physical-channels" => can_cluster_sum_physical_channels += 1,
                        "short-name" => inside_short_name = true,
                        "baudrate" => inside_baudrate = true,
                        "can-fd-baudrate" => inside_canfd_baudrate = true,
                        "can-frame-triggering" => {
                            inside_can_frame_triggering = true; 
                            cft_frame_ref = String::from("");
                            cft_identifier = String::from("");
                            cft_is_canfd = false;
                        }
                        "identifier" => inside_identifier = true,
                        "can-frame-tx-behavior" => inside_can_frame_tx_behavior = true,
                        "frame-ref" => inside_frame_ref = true,
                        "can-frame" => {
                            inside_can_frame = true;

                            frame_length = 0; 
                            pdu_ref = String::from(""); 
                        }
                        "frame-length" => inside_frame_length = true,
                        "pdu-ref" => inside_pdu_ref = true,
                        "cyclic-timing" => inside_cyclic_timing = true,
                        "time-period" => inside_time_period = true,
                        "value" => inside_value = true,
                        _ => {
                            if inside_pdu && depth == 5 && name.local_name.to_lowercase().as_str().contains("pdu") {
                                inside_pdu_element = true;
                                pdu_type = name.local_name;
                            }
                        }
                    };
                }

                Ok(XmlEvent::EndElement{ name }) => {
                    depth -= 1;

                    match name.local_name.to_lowercase().as_str() {
                        "autosar" => self.depth_check(depth, 1, "<autosar>", false),
                        "ar-packages" => self.depth_check(depth, 2, "<ar-packages>", false),
                        "ar-package" | "package" => {
                            self.depth_check(depth, 3, "<ar-package> or <package>", false);
                            inside_cluster = false;
                            inside_frame = false;
                            inside_pdu = false;
                            skip_package = false;
                        }
                        "can-cluster" => {
                            self.depth_check(depth, 5, "<can-cluster>", false);
                            inside_can_cluster = false;

                            let mut can_cluster: CanCluster = CanCluster {
                                name: can_cluster_name.to_string(),
                                baudrate: can_cluster_baudrate,
                                canfd_baudrate: can_cluster_canfd_baudrate,
                                sum_physical_channels: can_cluster_sum_physical_channels,
                                can_frame_triggerings: Vec::new()
                            };

                            can_cluster.can_frame_triggerings.append(&mut can_frame_triggerings);

                            can_clusters.push(can_cluster);
                        }
                        "short-name" => inside_short_name = false,
                        "baudrate" => inside_baudrate = false,
                        "can-fd-baudrate" => inside_canfd_baudrate = false,
                        "can-frame-triggering" => {
                            inside_can_frame_triggering = false; 

                            let can_frame_triggering: CanFrameTriggering = CanFrameTriggering {
                                frame_ref: cft_frame_ref.to_string(),
                                identifier: cft_identifier.to_string(),
                                is_canfd: cft_is_canfd
                            };

                            can_frame_triggerings.push(can_frame_triggering); 
                        }
                        "identifier" => inside_identifier = false,
                        "can-frame-tx-behavior" => inside_can_frame_tx_behavior = false,
                        "frame-ref" => inside_frame_ref = false,
                        "can-frame" => {
                            inside_can_frame = false;

                            let can_frame: CanFrame = CanFrame {
                                frame_length: frame_length,
                                pdu_ref: pdu_ref.to_string()
                            };

                            can_frames.push(can_frame);
                        }
                        "frame-length" => inside_frame_length = false,
                        "pdu-ref" => inside_pdu_ref = false,
                        "cyclic-timing" => inside_cyclic_timing = false,
                        "time-period" => inside_time_period = false,
                        "value" => inside_value = false,
                        _ => {
                            if inside_pdu && depth == 5 && name.local_name.to_lowercase().as_str() == pdu_type {
                                inside_pdu_element = false;

                                let pdu: Pdu = Pdu {
                                    pdu_type: pdu_type,
                                    name: pdu_name.to_string(),
                                    cyclic: pdu_cyclic,
                                    data: pdu_data.clone(),
                                    data_length: pdu_data_length,
                                    crc_offset: pdu_crc_offset,
                                    counter_offset: pdu_counter_offset 
                                };

                                pdus.push(pdu);
                                
                                pdu_type = String::from("");
                            }
                        }
                    };
                }

                Ok(XmlEvent::Characters( chars )) => {
                    if !skip_package {
                        if inside_short_name {
                            if depth == 4 {
                                let chars_lc = chars.to_lowercase().as_str().to_owned();
                                if targeted_packages.contains(&chars_lc) {
                                    // DEBUG
                                    println!("Found package of interest: {}", chars);
                                    // DEBUG END
                                    if &chars_lc == "cluster"  {
                                        inside_cluster = true;
                                    } else if &chars_lc == "frame"  {
                                        inside_frame = true;
                                    } else if &chars_lc == "pdu"  {
                                        inside_pdu = true;
                                    }

                                } else {
                                    // DEBUG
                                    println!("Found package not of interest: {}", chars);
                                    // DEBUG END
                                    skip_package = true;
                                }
                            } else if inside_can_cluster && depth == 6 {
                                can_cluster_name = chars; 
                            } else if inside_pdu && inside_pdu_element && depth == 6 {
                                pdu_name = chars;
                            }
                        } else if inside_can_cluster {
                            if inside_baudrate {
                                can_cluster_baudrate = chars.parse::<i32>().unwrap();
                            } else if inside_canfd_baudrate {
                                can_cluster_canfd_baudrate = chars.parse::<i32>().unwrap();
                            } else if inside_can_frame_triggering {
                                if inside_identifier {
                                    cft_identifier = chars; 
                                } else if inside_can_frame_tx_behavior {
                                    if chars.to_lowercase().as_str() == "can-fd" {
                                        cft_is_canfd = true;
                                    } 
                                } else if inside_frame_ref {
                                    cft_frame_ref = chars; 
                                }
                            }
                        } else if inside_frame && inside_can_frame {
                            if inside_frame_length {
                                frame_length = chars.parse::<i8>().unwrap();
                            } else if inside_pdu_ref {
                                pdu_ref = chars;
                            }
                        } else if inside_pdu && inside_pdu_element {
                            if inside_cyclic_timing && inside_time_period && inside_value {
                                pdu_cyclic = chars.parse::<f32>().unwrap();
                            }     
                        }
                    }
                }

                _ => {}
            }
        }
        
        // Resolve references and finalize structures
        // TODO
        

        // DEBUG
        println!("Got {} can-clusters", can_clusters.len());
        for can_cluster in can_clusters {
            println!("****");
            println!("Can-clusters -> name: {}, baudrate: {}, canfd_baudrate: {}, sum_physical_channels: {}", 
            can_cluster.name, can_cluster.baudrate, can_cluster.canfd_baudrate, can_cluster.sum_physical_channels);
            println!("Frames:");
            for can_frame_triggering in can_cluster.can_frame_triggerings {
                println!("Identifier: {}, is_canfd: {}, frame-ref: {}", can_frame_triggering.identifier, can_frame_triggering.is_canfd, can_frame_triggering.frame_ref);
            }
            println!("****");
        }

        let mut count = 0;
        for can_frame in can_frames {
            println!("CanFrame {} -> frame_length: {}, pdu_ref: {}", count, can_frame.frame_length, can_frame.pdu_ref); 
            count += 1;
            if count == 3 {
                break;
            }
        }
        // DEBUG END

        // warum cluster not of interest?
        
        println!("[+] Parsing done. Duration of parsing was: {:?}", start.elapsed());

        return true;
    }
}


fn main() {
    println!("[+] Starting openDuT ARXML parser over main method.");

    // config
    let file_name = "test.arxml";

    let arxml_parser: ArxmlParser = ArxmlParser {};

    let xml_reader: BufReader<File> = arxml_parser.create_reader(file_name);

    arxml_parser.parse_data(xml_reader);
}
