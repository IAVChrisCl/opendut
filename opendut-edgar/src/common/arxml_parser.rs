use std::fs::File;
use std::io::BufReader;

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
    - increase parsing speed by skipping ar-packages not of interest
    - support multiple can-cluster variants and physical channels
    - Currently using xml-rs. Use quick_xml when applicable

- Code inside DEBUG comments will be removed at a later stage
*/


// Future restbus simulation structure used to setup and control restbus simulation. Will be moved to seprarate source code file.
pub struct RestbusSimulation {

}

// Can frames of a can cluster
// Some values will be skipped when parsing. These will be filled correctly when resolving references
pub struct CanFrame {
    frame_ref: String,
    identifier: String,
    is_canfd: bool,
    //dlc: i8,
    //...
}

// Can cluster structure
pub struct CanCluster {
    name: String,
    baudrate: i32,
    canfd_baudrate: i32,
    sum_physical_channels: i32,
    can_frames: Vec<CanFrame>
    // config?
}

// Parser structure
pub struct ArxmlParser {
}

impl ArxmlParser {
    // Read file and return BufReader
    pub fn read_file(&self, file_name: &str) -> BufReader<File> {
        println!("[+] Called Parser.read_file with argument {}", file_name);
        println!("[+] Reading ARXML file. Using file name: {}", file_name);

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
    pub fn parse_file(&self, xml_reader: BufReader<File>) -> bool {
        let mut can_clusters: Vec<CanCluster> = Vec::new();

        let parser = EventReader::new(xml_reader);

        // DEBUG
        let mut count = 0;
        // DEBUG END

        let mut depth = 0; // 1 = autosar, 2 = ar-packages, 3 = ar-package/package

        // FLAGS used for mainly detecting if we are inside of XML elements
        let mut inside_short_name= false;
        let mut inside_cluster = false;
        let mut inside_can_cluster = false;
        let mut inside_baudrate= false;
        let mut inside_canfd_baudrate= false;
        let mut no_cluster_yet = true;
        let mut inside_can_frame_triggering= false;
        let mut inside_identifier= false;
        let mut inside_can_frame_tx_behavior = false;
        let mut inside_frame_ref = false;

        // Temporary values to store CAN cluster data
        let mut cluster_name: String = String::from(""); 
        let mut baudrate: i32 = 0;
        let mut canfd_baudrate: i32 = 0;
        let mut sum_physical_channels: i32 = 0;
        let mut can_frames: Vec<CanFrame> = Vec::new();

        // Temporary values to store CAN frame data
        let mut frame_ref: String = String::from("");
        let mut identifier: String = String::from("");
        let mut is_canfd: bool = false;

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
                        "ar-package" | "package" => {
                            self.depth_check(depth, 3, "<ar-packages> or <package>", true);
                        }
                        "can-cluster" => {
                            self.depth_check(depth, 5, "<can-cluster>", true);
                            if inside_cluster {
                                inside_can_cluster = true;

                                cluster_name = String::from(""); 
                                baudrate = 0;
                                canfd_baudrate = 0;
                                sum_physical_channels = 0;
                                can_frames = Vec::new();
                            }
                        }
                        "physical-channels" => sum_physical_channels += 1,
                        "short-name" => inside_short_name = true,
                        "baudrate" => inside_baudrate = true,
                        "can-fd-baudrate" => inside_canfd_baudrate = true,
                        "can-frame-triggering" => {
                            inside_can_frame_triggering = true; 
                            frame_ref = String::from("");
                            identifier = String::from("");
                            is_canfd = false;
                        }
                        "identifier" => inside_identifier = true,
                        "can-frame-tx-behavior" => inside_can_frame_tx_behavior = true,
                        "frame-ref" => inside_frame_ref = true,
                        _ => {}
                    };
                }

                Ok(XmlEvent::EndElement{ name }) => {
                    depth -= 1;

                    match name.local_name.to_lowercase().as_str() {
                        "autosar" => self.depth_check(depth, 1, "<autosar>", false),
                        "ar-packages" => self.depth_check(depth, 2, "<ar-packages>", false),
                        "ar-package" | "package" => {
                            self.depth_check(depth, 3, "<ar-packages> or <package>", false);
                            inside_cluster = false;
                        }
                        "can-cluster" => {
                            self.depth_check(depth, 5, "<can-cluster>", false);
                            inside_can_cluster = false;

                            let mut can_cluster: CanCluster = CanCluster {
                                name: cluster_name.to_string(),
                                baudrate: baudrate,
                                canfd_baudrate: canfd_baudrate,
                                sum_physical_channels: sum_physical_channels,
                                can_frames: Vec::new()
                            };

                            can_cluster.can_frames.append(&mut can_frames);

                            can_clusters.push(can_cluster);
                        }
                        "short-name" => inside_short_name = false,
                        "baudrate" => inside_baudrate = false,
                        "can-fd-baudrate" => inside_canfd_baudrate = false,
                        "can-frame-triggering" => {
                            inside_can_frame_triggering = false; 

                            let can_frame: CanFrame = CanFrame {
                                frame_ref: frame_ref.to_string(),
                                identifier: identifier.to_string(),
                                is_canfd: is_canfd
                            };

                            can_frames.push(can_frame); 
                        }
                        "identifier" => inside_identifier = false,
                        "can-frame-tx-behavior" => inside_can_frame_tx_behavior = false,
                        "frame-ref" => inside_frame_ref = false,
                        _ => {}
                    }; 
                }

                Ok(XmlEvent::Characters( chars )) => {
                    if inside_short_name {
                        if no_cluster_yet && depth == 4 && chars.to_lowercase().as_str() == "cluster"  {
                            println!("found cluster");
                            inside_cluster = true;
                            no_cluster_yet = false;
                            
                        } else if inside_can_cluster && depth == 6 {
                            cluster_name = chars; 
                        }
                    } else if inside_can_cluster {
                        if inside_baudrate {
                            baudrate = chars.parse::<i32>().unwrap();
                        } else if inside_canfd_baudrate {
                            canfd_baudrate = chars.parse::<i32>().unwrap();
                        } else if inside_can_frame_triggering {
                            if inside_identifier {
                                identifier = chars; 
                            } else if inside_can_frame_tx_behavior {
                                if chars.to_lowercase().as_str() == "can-fd" {
                                    is_canfd = true;
                                } 
                            } else if inside_frame_ref {
                                frame_ref = chars; 
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
            for can_frame in can_cluster.can_frames {
                println!("Identifier: {}, is_canfd: {}, frame-ref: {}", can_frame.identifier, can_frame.is_canfd, can_frame.frame_ref);
            }
            println!("****");
        }
        // DEBUG END

        return true;
    }
}


fn main() {
    println!("[+] Starting openDuT ARXML parser over main method.");

    // config
    let file_name = "test.arxml";

    let arxml_parser: ArxmlParser = ArxmlParser {};

    let xml_reader: BufReader<File> = arxml_parser.read_file(file_name);

    arxml_parser.parse_file(xml_reader);
}
