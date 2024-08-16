use quartz_nbt::io::Flavor;
use schematic_converter::converters::{litematic_to_schematic, schematic_to_litematic};
use std::io::Cursor;
use quartz_nbt::{NbtCompound, NbtTag};

#[test]
fn test_litematic_to_schematic_conversion() {
    let sample_litematic = include_bytes!("test_schematics/sample.litematic");
    let mut output = Vec::new();

    litematic_to_schematic(Cursor::new(sample_litematic), &mut output).unwrap();

    assert!(!output.is_empty(), "Output should not be empty");

    let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(&output), Flavor::Uncompressed).unwrap();

    assert!(nbt.contains_key("Width"), "Output should contain 'Width' tag");
    assert!(nbt.contains_key("Height"), "Output should contain 'Height' tag");
    assert!(nbt.contains_key("Length"), "Output should contain 'Length' tag");
    assert!(nbt.contains_key("BlockData"), "Output should contain 'BlockData' tag");
    assert!(nbt.contains_key("Palette"), "Output should contain 'Palette' tag");

    println!("Converted Schematic NBT structure: {:#?}", nbt);
}
//
// #[test]
// fn test_schematic_to_litematic_conversion() {
//     let mut schematic_nbt = NbtCompound::new();
//     schematic_nbt.insert("Width", NbtTag::Short(2));
//     schematic_nbt.insert("Height", NbtTag::Short(2));
//     schematic_nbt.insert("Length", NbtTag::Short(2));
//     schematic_nbt.insert("BlockData", NbtTag::ByteArray(vec![0, 0, 0, 0, 1, 1, 2, 1]));
//     let mut palette = NbtCompound::new();
//     palette.insert("minecraft:air", NbtTag::Int(0));
//     palette.insert("minecraft:stone", NbtTag::Int(1));
//     palette.insert("minecraft:dirt", NbtTag::Int(2));
//     schematic_nbt.insert("Palette", NbtTag::Compound(palette));
//
//     let mut schematic_data = Vec::new();
//     quartz_nbt::io::write_nbt(&mut schematic_data, None, &schematic_nbt, Flavor::Uncompressed).unwrap();
//
//     let mut litematic_output = Vec::new();
//     schematic_to_litematic(Cursor::new(schematic_data), &mut litematic_output).unwrap();
//
//     assert!(!litematic_output.is_empty(), "Output should not be empty");
//
//     let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(litematic_output), Flavor::Uncompressed).unwrap();
//
//     assert!(nbt.contains_key("Metadata"), "Output should contain 'Metadata' tag");
//     assert!(nbt.contains_key("Regions"), "Output should contain 'Regions' tag");
//     assert!(nbt.contains_key("MinecraftDataVersion"), "Output should contain 'MinecraftDataVersion' tag");
//
//     // println!("Converted Litematic NBT structure: {:#?}", nbt);
// }
//
// #[test]
// fn test_roundtrip_conversion() {
//     let sample_litematic = include_bytes!("test_schematics/sample.litematic");
//
//     let mut schematic_data = Vec::new();
//     litematic_to_schematic(Cursor::new(sample_litematic), &mut schematic_data).unwrap();
//
//     let mut roundtrip_litematic = Vec::new();
//     schematic_to_litematic(Cursor::new(schematic_data), &mut roundtrip_litematic).unwrap();
//
//     let parse_litematic = |data: &[u8]| -> NbtCompound {
//         let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(data), Flavor::Uncompressed).unwrap();
//         nbt
//     };
//
//     let original_nbt = parse_litematic(sample_litematic);
//     let roundtrip_nbt = parse_litematic(&roundtrip_litematic);
//
//     assert_eq!(original_nbt, roundtrip_nbt, "Roundtrip conversion should preserve NBT structure");
// }

#[test]
fn print_sample_contents() {
    let sample_schem = include_bytes!("test_schematics/sample.schem");
    let sample_litematic = include_bytes!("test_schematics/sample.litematic");

    println!("Contents of sample.schem:");
    print_nbt_contents(sample_schem, true);

    println!("\nContents of sample.litematic:");
    print_nbt_contents(sample_litematic, true);
}

fn print_nbt_contents(data: &[u8], is_compressed: bool) {
    let nbt = if is_compressed {
        println!("Decompressing data...");
        let mut decoder = flate2::read::GzDecoder::new(Cursor::new(data));
        let mut decompressed = Vec::new();
        std::io::copy(&mut decoder, &mut decompressed).unwrap();
        let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed).unwrap();
        nbt
    } else {
        println!("Reading uncompressed data...");
        let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(data), Flavor::Uncompressed).unwrap();
        nbt
    };

    println!("{:#?}", nbt);
}