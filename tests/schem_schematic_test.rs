use quartz_nbt::io::Flavor;
use schematic_converter::converters::{schem_to_schematic, schematic_to_schem};
use std::io::Cursor;
use quartz_nbt::{NbtCompound, NbtTag};

#[test]
fn test_schem_to_schematic_conversion() {
    let sample_schem = include_bytes!("test_schematics/sample.schem");
    let mut output = Vec::new();

    schem_to_schematic(Cursor::new(sample_schem), &mut output).unwrap();

    assert!(!output.is_empty(), "Output should not be empty");

    let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(&output), Flavor::Uncompressed).unwrap();

    assert!(nbt.contains_key("Width"), "Output should contain 'Width' tag");
    assert!(nbt.contains_key("Height"), "Output should contain 'Height' tag");
    assert!(nbt.contains_key("Length"), "Output should contain 'Length' tag");
    assert!(nbt.contains_key("BlockData"), "Output should contain 'BlockData' tag");
    assert!(nbt.contains_key("Palette"), "Output should contain 'Palette' tag");


    // println!("Decompressed NBT structure: {:#?}", nbt);
}

#[test]
fn test_schematic_to_schem_conversion() {
    let mut schematic_nbt = NbtCompound::new();
    schematic_nbt.insert("Width", NbtTag::Short(2));
    schematic_nbt.insert("Height", NbtTag::Short(2));
    schematic_nbt.insert("Length", NbtTag::Short(2));
    schematic_nbt.insert("BlockData", NbtTag::ByteArray(vec![0, 0, 0, 0, 1, 1, 2, 1]));
    let mut palette = NbtCompound::new();
    palette.insert("minecraft:air", NbtTag::Int(0));
    palette.insert("minecraft:stone", NbtTag::Int(1));
    palette.insert("minecraft:dirt", NbtTag::Int(2));
    schematic_nbt.insert("Palette", NbtTag::Compound(palette));

    let mut schematic_data = Vec::new();
    quartz_nbt::io::write_nbt(&mut schematic_data, None, &schematic_nbt, Flavor::Uncompressed).unwrap();

    let mut schem_output = Vec::new();
    schematic_to_schem(Cursor::new(schematic_data), &mut schem_output).unwrap();

    assert!(!schem_output.is_empty(), "Output should not be empty");

    let mut decoder = flate2::read::GzDecoder::new(Cursor::new(schem_output));
    let mut decompressed = Vec::new();
    std::io::copy(&mut decoder, &mut decompressed).unwrap();
    let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed).unwrap();

    assert!(nbt.contains_key("Width"), "Output should contain 'Width' tag");
    assert!(nbt.contains_key("Height"), "Output should contain 'Height' tag");
    assert!(nbt.contains_key("Length"), "Output should contain 'Length' tag");
    assert!(nbt.contains_key("BlockData"), "Output should contain 'BlockData' tag");
    assert!(nbt.contains_key("Palette"), "Output should contain 'Palette' tag");

    // println!("Decompressed NBT structure: {:#?}", nbt);
}

#[test]
fn test_roundtrip_conversion() {
    let sample_schem = include_bytes!("test_schematics/sample.schem");

    let mut schematic_data = Vec::new();
    schem_to_schematic(Cursor::new(sample_schem), &mut schematic_data).unwrap();

    let mut roundtrip_schem = Vec::new();
    schematic_to_schem(Cursor::new(schematic_data), &mut roundtrip_schem).unwrap();

    let parse_schem = |data: &[u8]| -> NbtCompound {
        let mut decoder = flate2::read::GzDecoder::new(Cursor::new(data));
        let mut decompressed = Vec::new();
        std::io::copy(&mut decoder, &mut decompressed).unwrap();
        let (nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed).unwrap();
        nbt
    };

    let original_nbt = parse_schem(sample_schem);
    let roundtrip_nbt = parse_schem(&roundtrip_schem);

    assert_eq!(original_nbt, roundtrip_nbt, "Roundtrip conversion should preserve NBT structure");
}


