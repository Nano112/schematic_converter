use schematic_converter::{SchematicConverter, SchematicFormat};

#[test]
fn test_litematic_to_schematic_to_schem() {
    let sample_litematic = include_bytes!("test_schematics/big_quarry.litematic");

    std::fs::create_dir_all("tests/outputs").unwrap();

    let converter = SchematicConverter::new();


    // Convert Litematic to Schematic
    let schematic_output = converter.convert(
        sample_litematic,
        SchematicFormat::Litematic,
        SchematicFormat::Schematic
    ).expect("Failed to convert Litematic to Schematic");

    // Convert Schematic to Schem
    let schem_output = converter.convert(
        &schematic_output,
        SchematicFormat::Schematic,
        SchematicFormat::Schem
    ).expect("Failed to convert Schematic to Schem");

    std::fs::write("tests/outputs/big_quarry.schem", schem_output).unwrap();
}