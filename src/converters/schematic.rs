use std::io::{Read, Write};
use flate2::write::GzEncoder;
use flate2::Compression;
use quartz_nbt::{NbtCompound, NbtList, NbtTag};

pub fn schematic_to_schem<R: Read, W: Write>(mut input: R, output: W) -> Result<(), Box<dyn std::error::Error>> {
    // Create a gzip encoder
    let mut encoder = GzEncoder::new(output, Compression::default());

    // Copy the input directly to the gzip encoder
    std::io::copy(&mut input, &mut encoder)?;

    // Finish the compression
    encoder.finish()?;

    Ok(())
}

pub fn schematic_to_litematic<R: Read, W: Write>(mut input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    let (schematic_nbt, _) = quartz_nbt::io::read_nbt(&mut input, quartz_nbt::io::Flavor::Uncompressed)?;

    let mut litematic_nbt = NbtCompound::new();

    convert_metadata_to_litematic(&schematic_nbt, &mut litematic_nbt)?;
    convert_palette_to_litematic(&schematic_nbt, &mut litematic_nbt)?;
    pack_block_data_to_litematic(&schematic_nbt, &mut litematic_nbt)?;
    convert_entities_to_litematic(&schematic_nbt, &mut litematic_nbt)?;

    quartz_nbt::io::write_nbt(&mut output, None, &litematic_nbt, quartz_nbt::io::Flavor::Uncompressed)?;

    Ok(())
}

fn convert_metadata_to_litematic(schematic: &NbtCompound, litematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    let mut metadata = NbtCompound::new();
    let mut enclosing_size = NbtCompound::new();

    enclosing_size.insert("x", schematic.get::<_, &NbtTag>("Width")?.clone());
    enclosing_size.insert("y", schematic.get::<_, &NbtTag>("Height")?.clone());
    enclosing_size.insert("z", schematic.get::<_, &NbtTag>("Length")?.clone());

    metadata.insert("EnclosingSize", NbtTag::Compound(enclosing_size));

    if let Ok(NbtTag::String(author)) = schematic.get::<_, &NbtTag>("Author") {
        metadata.insert("Author", NbtTag::String(author.clone()));
    }

    if let Ok(NbtTag::String(description)) = schematic.get::<_, &NbtTag>("Description") {
        metadata.insert("Description", NbtTag::String(description.clone()));
    }

    litematic.insert("Metadata", NbtTag::Compound(metadata));

    if let Ok(NbtTag::Int(data_version)) = schematic.get::<_, &NbtTag>("DataVersion") {
        litematic.insert("MinecraftDataVersion", NbtTag::Int(*data_version));
    }

    Ok(())
}

fn convert_palette_to_litematic(schematic: &NbtCompound, litematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    let mut regions = NbtCompound::new();
    let mut region = NbtCompound::new();
    let mut block_state_palette = NbtList::new();

    if let Ok(NbtTag::Compound(palette)) = schematic.get::<_, &NbtTag>("Palette") {
        for (full_name, _) in palette.inner().iter() {
            let mut block_state = NbtCompound::new();
            let mut name = full_name.clone();
            let mut properties = NbtCompound::new();

            if let Some(bracket_index) = full_name.find('[') {
                name = full_name[..bracket_index].to_string();
                let props_str = &full_name[bracket_index+1..full_name.len()-1];
                for prop in props_str.split(',') {
                    let mut key_value = prop.split('=');
                    if let (Some(key), Some(value)) = (key_value.next(), key_value.next()) {
                        properties.insert(key, NbtTag::String(value.to_string()));
                    }
                }
                block_state.insert("Properties", NbtTag::Compound(properties));
            }

            block_state.insert("Name", NbtTag::String(name));
            block_state_palette.push(NbtTag::Compound(block_state));
        }
    }

    region.insert("BlockStatePalette", NbtTag::List(block_state_palette));
    regions.insert("main", NbtTag::Compound(region));
    litematic.insert("Regions", NbtTag::Compound(regions));

    Ok(())
}

fn pack_block_data_to_litematic(schematic: &NbtCompound, litematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(NbtTag::ByteArray(block_data)) = schematic.get::<_, &NbtTag>("BlockData") {
        if let Ok(Some(NbtTag::Compound(ref mut regions))) = litematic.get_mut("Regions") {
            if let Ok(Some(NbtTag::Compound(ref mut region))) = regions.get_mut("main") {
                let width = match schematic.get::<_, &NbtTag>("Width")? {
                    NbtTag::Short(w) => *w as usize,
                    _ => return Err("Invalid Width".into()),
                };
                let height = match schematic.get::<_, &NbtTag>("Height")? {
                    NbtTag::Short(h) => *h as usize,
                    _ => return Err("Invalid Height".into()),
                };
                let length = match schematic.get::<_, &NbtTag>("Length")? {
                    NbtTag::Short(l) => *l as usize,
                    _ => return Err("Invalid Length".into()),
                };

                let bits_per_block = (block_data.len() as f64 / (width * height * length) as f64).ceil() as usize;
                let mask = (1 << bits_per_block) - 1;

                let mut block_states = Vec::new();
                let mut current_long = 0i64;
                let mut bits_filled = 0;

                for &block in block_data {
                    current_long |= (block as i64 & mask) << bits_filled;
                    bits_filled += bits_per_block;

                    if bits_filled >= 64 {
                        block_states.push(current_long);
                        current_long = 0;
                        bits_filled = 0;
                    }
                }

                if bits_filled > 0 {
                    block_states.push(current_long);
                }

                region.insert("BlockStates", NbtTag::LongArray(block_states));

                let mut size = NbtCompound::new();
                size.insert("x", NbtTag::Short(width as i16));
                size.insert("y", NbtTag::Short(height as i16));
                size.insert("z", NbtTag::Short(length as i16));
                region.insert("Size", NbtTag::Compound(size));
            }
        }
    }
    Ok(())
}

fn convert_entities_to_litematic(schematic: &NbtCompound, litematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(Some(NbtTag::Compound(ref mut regions))) = litematic.get_mut("Regions") {
        if let Ok(Some(NbtTag::Compound(ref mut region))) = regions.get_mut("main") {
            if let Ok(entities) = schematic.get::<_, &NbtTag>("Entities") {
                region.insert("Entities", entities.clone());
            }
            if let Ok(tile_entities) = schematic.get::<_, &NbtTag>("TileEntities") {
                region.insert("TileEntities", tile_entities.clone());
            }
        }
    }
    Ok(())
}