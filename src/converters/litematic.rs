use std::io::{Cursor, Read, Write};
use quartz_nbt::{NbtCompound, NbtTag, NbtList};
use quartz_nbt::io::Flavor;

pub fn litematic_to_schematic<R: Read, W: Write>(mut input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut decompressed = Vec::new();
    std::io::copy(&mut decoder, &mut decompressed)?;

    let (litematic_nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed)?;

    let mut schematic_nbt = NbtCompound::new();

    convert_metadata_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    convert_palette_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    unpack_block_data_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    convert_entities_to_schematic(&litematic_nbt, &mut schematic_nbt)?;

    quartz_nbt::io::write_nbt(&mut output, None, &schematic_nbt, quartz_nbt::io::Flavor::Uncompressed)?;

    Ok(())
}

fn convert_metadata_to_schematic(litematic: &NbtCompound, schematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(NbtTag::Compound(metadata)) = litematic.get::<_, &NbtTag>("Metadata") {
        if let Ok(Some(NbtTag::Compound(enclosing_size))) = metadata.get("EnclosingSize") {
            schematic.insert("Width", enclosing_size.get::<_, &NbtTag>("x")?.clone());
            schematic.insert("Height", enclosing_size.get::<_, &NbtTag>("y")?.clone());
            schematic.insert("Length", enclosing_size.get::<_, &NbtTag>("z")?.clone());
        } else {
            return Err("Missing EnclosingSize in Metadata".into());
        }

        if let Ok(NbtTag::String(author)) = metadata.get::<_, &NbtTag>("Author") {
            schematic.insert("Author", NbtTag::String(author.clone()));
        }

        if let Ok(NbtTag::String(description)) = metadata.get::<_, &NbtTag>("Description") {
            schematic.insert("Description", NbtTag::String(description.clone()));
        }
    } else {
        return Err("Missing or invalid Metadata".into());
    }

    if let Ok(NbtTag::Int(data_version)) = litematic.get::<_, &NbtTag>("MinecraftDataVersion") {
        schematic.insert("DataVersion", NbtTag::Int(*data_version));
    }

    Ok(())
}

fn convert_palette_to_schematic(litematic: &NbtCompound, schematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    let mut schematic_palette = NbtCompound::new();

    if let Ok(NbtTag::Compound(regions)) = litematic.get::<_, &NbtTag>("Regions") {
        if let Some(NbtTag::Compound(region)) = regions.inner().values().next() {
            if let Ok(NbtTag::List(block_state_palette)) = region.get::<_, &NbtTag>("BlockStatePalette") {
                for (i, block_state) in block_state_palette.iter().enumerate() {
                    if let NbtTag::Compound(block_state_compound) = block_state {
                        if let Ok(Some(NbtTag::String(name))) = block_state_compound.get("Name") {
                            let mut full_name = name.clone();
                            if let Ok(Some(NbtTag::Compound(properties))) = block_state_compound.get("Properties") {
                                full_name.push('[');
                                for (key, value) in properties.inner().iter() {
                                    let value_str = match value {
                                        NbtTag::String(s) => s.clone(),
                                        NbtTag::Byte(b) => b.to_string(),
                                        NbtTag::Short(s) => s.to_string(),
                                        NbtTag::Int(i) => i.to_string(),
                                        NbtTag::Long(l) => l.to_string(),
                                        NbtTag::Float(f) => f.to_string(),
                                        NbtTag::Double(d) => d.to_string(),
                                        _ => return Err("Unexpected property value type".into()),
                                    };
                                    full_name.push_str(&format!("{}={},", key, value_str));
                                }
                                full_name.pop(); // Remove last comma
                                full_name.push(']');
                            }
                            schematic_palette.insert(full_name, NbtTag::Int(i as i32));
                        }
                    }
                }
            }
        }
    }

    schematic.insert("Palette", NbtTag::Compound(schematic_palette));
    schematic.insert("Version", NbtTag::Int(2));
    Ok(())
}

fn unpack_block_data_to_schematic(litematic: &NbtCompound, schematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(NbtTag::Compound(regions)) = litematic.get::<_, &NbtTag>("Regions") {
        if let Some(NbtTag::Compound(region)) = regions.inner().values().next() {
            if let Ok(NbtTag::LongArray(block_states)) = region.get::<_, &NbtTag>("BlockStates") {
                let mut block_data = Vec::new();

                let size = region.get::<_, &NbtTag>("Size").map_err(|e| {
                    format!("Failed to get 'Size' tag from region: {}", e)
                })?;

                if let NbtTag::Compound(size_compound) = size {

                    let width = match size_compound.get::<_, &NbtTag>("x") {
                        Ok(NbtTag::Int(w)) => *w,
                        Ok(_) => return Err("Invalid 'x' size type; expected Int.".into()),
                        Err(e) => return Err(format!("Error accessing 'x' size: {}", e).into()),
                    };

                    let height = match size_compound.get::<_, &NbtTag>("y") {
                        Ok(NbtTag::Int(h)) => *h,
                        Ok(_) => return Err("Invalid 'y' size type; expected Int.".into()),
                        Err(e) => return Err(format!("Error accessing 'y' size: {}", e).into()),
                    };

                    let length = match size_compound.get::<_, &NbtTag>("z") {
                        Ok(NbtTag::Int(l)) => *l,
                        Ok(_) => return Err("Invalid 'z' size type; expected Int.".into()),
                        Err(e) => return Err(format!("Error accessing 'z' size: {}", e).into()),
                    };

                    if width == 0 || height == 0 || length == 0 {
                        return Err(format!("Invalid region size: width={}, height={}, length={}", width, height, length).into());
                    }

                    let volume = (width * height * length) as usize;

                    // Calculate bits_per_block based on the BlockStatePalette length
                    if let Ok(NbtTag::List(palette)) = region.get::<_, &NbtTag>("BlockStatePalette") {
                        let palette_length = palette.len();
                        let bits_per_block = (palette_length as f64).log2().ceil() as usize;
                        let mask = (1u64 << bits_per_block) - 1;

                        let mut current_long_index = 0;
                        let mut current_long_offset = 0;

                        while block_data.len() < volume {
                            let current_value = block_states[current_long_index] as u64 >> current_long_offset;
                            let block_state_index = (current_value & mask) as u8;

                            block_data.push(block_state_index);

                            current_long_offset += bits_per_block;
                            if current_long_offset >= 64 {
                                current_long_index += 1;
                                current_long_offset -= 64;
                            }
                        }

                        // Convert Vec<u8> to Vec<i8>
                        let block_data_i8: Vec<i8> = block_data.into_iter().map(|x| x as i8).collect();

                        schematic.insert("BlockData", NbtTag::ByteArray(block_data_i8));
                    } else {
                        return Err("Failed to get 'BlockStatePalette' tag from region.".into());
                    }
                } else {
                    return Err("Expected 'Size' tag to be a Compound, but found different type.".into());
                }
            } else {
                return Err("Failed to find or invalid 'BlockStates' tag in region.".into());
            }
        } else {
            return Err("Failed to find or invalid 'Region' tag in 'Regions'.".into());
        }
    } else {
        return Err("Failed to find or invalid 'Regions' tag.".into());
    }
    Ok(())
}






fn convert_entities_to_schematic(litematic: &NbtCompound, schematic: &mut NbtCompound) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(NbtTag::Compound(regions)) = litematic.get::<_, &NbtTag>("Regions") {
        if let Some(NbtTag::Compound(region)) = regions.inner().values().next() {
            if let Ok(Some(entities)) = region.get("Entities") {
                schematic.insert("Entities", entities.clone());
            }
            if let Ok(Some(tile_entities)) = region.get("TileEntities") {
                schematic.insert("TileEntities", tile_entities.clone());
            }
        }
    }
    Ok(())
}