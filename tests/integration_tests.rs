use quartz_nbt::io::Flavor;
use schematic_converter::converters::{litematic_to_schematic, schematic_to_litematic, schematic_to_schem};
use std::io::Cursor;



// load the sample litematic file convert to schematic and then to schem 
#[test]
fn test_litematic_to_schematic() {
    let sample_litematic = include_bytes!("test_schematics/quary.litematic");

    // ensure the folder exists
    std::fs::create_dir_all("tests/outputs").unwrap();
    let mut output = Vec::new();
    let mut output2 = Vec::new();

    litematic_to_schematic(Cursor::new(sample_litematic), &mut output).unwrap();
    schematic_to_schem(Cursor::new(output), &mut output2).unwrap();

    //save both the schematic and the schem
    // std::fs::write("tests/outputs/sample.schematic", output).unwrap();
    std::fs::write("tests/outputs/quary.schem", output2).unwrap();
    // println!("Volume: {}, Bits per block: {}", volume, bits_per_block);
}