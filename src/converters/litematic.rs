use std::io::{Cursor, Read, Write};
use log::debug;
use quartz_nbt::{NbtCompound, NbtTag};
use quartz_nbt::io::Flavor;

pub fn litematic_to_schematic<R: Read, W: Write>( input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut decompressed = Vec::new();
    std::io::copy(&mut decoder, &mut decompressed)?;

    let (litematic_nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed)?;

    let mut schematic_nbt = NbtCompound::new();
    debug!("litematic: {:?}", litematic_nbt);
    convert_metadata_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    convert_palette_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    unpack_block_data_to_schematic(&litematic_nbt, &mut schematic_nbt)?;
    convert_entities_to_schematic(&litematic_nbt, &mut schematic_nbt)?;

    quartz_nbt::io::write_nbt(&mut output, None, &schematic_nbt, quartz_nbt::io::Flavor::Uncompressed)?;
    //debug print the output
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
    debug!("litematic: {:?}", litematic);
    if let Ok(NbtTag::Compound(regions)) = litematic.get::<_, &NbtTag>("Regions") {
        debug!("regions: {:?}", regions);
        if let Some(NbtTag::Compound(region)) = regions.inner().values().next() {
            if let Ok(NbtTag::List(block_state_palette)) = region.get::<_, &NbtTag>("BlockStatePalette") {
                for (i, block_state) in block_state_palette.iter().enumerate() {
                    if let NbtTag::Compound(block_state_compound) = block_state {
                        if let Ok(Some(NbtTag::String(name))) = block_state_compound.get("Name") {
                            let mut full_name = name.clone();
                            if let Ok(Some(NbtTag::Compound(properties))) = block_state_compound.get("Properties") {
                                debug!("properties: {:?}", properties);
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
            let size = region.get::<_, &NbtTag>("Size")
                .map_err(|e| format!("Failed to get 'Size' tag from region: {}", e))?;

            if let NbtTag::Compound(size_compound) = size {
                let width = size_compound.get::<_, i32>("x")?;
                let height = size_compound.get::<_, i32>("y")?;
                let length = size_compound.get::<_, i32>("z")?;

                if width == 0 || height == 0 || length == 0 {
                    return Err(format!("Invalid region size: width={}, height={}, length={}", width, height, length).into());
                }

                let volume = (width.abs() * height.abs() * length.abs()) as usize;

                schematic.insert("Width", NbtTag::Short(width.abs() as i16));
                schematic.insert("Height", NbtTag::Short(height.abs() as i16));
                schematic.insert("Length", NbtTag::Short(length.abs() as i16));

                // Use the existing palette from convert_palette_to_schematic
                let palette = schematic.get::<_, &NbtCompound>("Palette")
                    .map_err(|_| "Palette not found in schematic. Make sure convert_palette_to_schematic is called before this function.")?;
                let palette_length = palette.len();
                schematic.insert("PaletteMax", NbtTag::Int(palette_length as i32));

                // Process blocks
                if let Ok(NbtTag::LongArray(block_states)) = region.get::<_, &NbtTag>("BlockStates") {
                    let bits_per_block = if palette_length <= 1 {
                        1
                    } else {
                        std::cmp::max((palette_length as f64).log2().ceil() as usize, 1)
                    };
                    let mask = (1u64 << bits_per_block) - 1;

                    let mut block_data = Vec::new();
                    let mut bit_buffer = 0u64;
                    let mut bits_in_buffer = 0;
                    let mut block_states_iter = block_states.iter();

                    for _ in 0..volume {
                        while bits_in_buffer < bits_per_block {
                            match block_states_iter.next() {
                                Some(&long_value) => {
                                    bit_buffer |= (long_value as u64) << bits_in_buffer;
                                    bits_in_buffer += 64;
                                }
                                None => return Err("BlockStates array is too short.".into()),
                            }
                        }

                        let block_state_index = bit_buffer & mask;
                        bit_buffer >>= bits_per_block;
                        bits_in_buffer -= bits_per_block;

                        // Encode block_state_index as VarInt
                        let mut varint = block_state_index;
                        loop {
                            let mut byte = (varint & 0x7F) as u8;
                            varint >>= 7;
                            if varint != 0 {
                                byte |= 0x80;
                            }
                            block_data.push(byte as i8);
                            if varint == 0 {
                                break;
                            }
                        }
                    }

                    schematic.insert("BlockData", NbtTag::ByteArray(block_data));
                } else {
                    return Err("Failed to find or invalid 'BlockStates' tag in region.".into());
                }
            } else {
                return Err("Expected 'Size' tag to be a Compound, but found different type.".into());
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