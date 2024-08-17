use std::io::{Cursor, Read, Write};
use log::debug;
use quartz_nbt::{NbtCompound, NbtList, NbtTag};
use quartz_nbt::io::Flavor;

pub fn litematic_to_schematic<R: Read, W: Write>(input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = flate2::read::GzDecoder::new(input);
    let mut decompressed = Vec::new();
    std::io::copy(&mut decoder, &mut decompressed)?;

    let (litematic_nbt, _) = quartz_nbt::io::read_nbt(&mut Cursor::new(decompressed), Flavor::Uncompressed)?;

    let mut schematic_nbt = NbtCompound::new();
    debug!("litematic: {:?}", litematic_nbt);

    // Handle metadata
    convert_metadata_to_schematic(&litematic_nbt, &mut schematic_nbt)?;

    // Get all regions
    let regions = litematic_nbt.get::<_, &NbtCompound>("Regions")
        .map_err(|_| "Missing Regions compound")?;

    // Calculate overall dimensions and offsets
    let (width, height, length, min_x, min_y, min_z) = calculate_overall_dimensions(regions)?;

    schematic_nbt.insert("Width", NbtTag::Short(width as i16));
    schematic_nbt.insert("Height", NbtTag::Short(height as i16));
    schematic_nbt.insert("Length", NbtTag::Short(length as i16));
    schematic_nbt.insert("Offset", NbtTag::IntArray(vec![min_x, min_y, min_z]));

    // Create unified palette and block data
    let (palette, block_data) = create_unified_palette_and_data(regions, width, height, length, min_x, min_y, min_z)?;

    schematic_nbt.insert("Palette", NbtTag::Compound(palette));
    schematic_nbt.insert("PaletteMax", NbtTag::Int(palette.len() as i32));
    schematic_nbt.insert("BlockData", NbtTag::ByteArray(block_data));

    // Handle entities and block entities
    let (entities, block_entities) = collect_entities_and_block_entities(regions, min_x, min_y, min_z)?;
    if !entities.is_empty() {
        schematic_nbt.insert("Entities", NbtList::from(entities));
    }
    if !block_entities.is_empty() {
        schematic_nbt.insert("BlockEntities", NbtList::from(block_entities));
    }

    schematic_nbt.insert("Version", NbtTag::Int(2)); // Sponge Schematic version 2

    quartz_nbt::io::write_nbt(&mut output, None, &schematic_nbt, quartz_nbt::io::Flavor::Uncompressed)?;
    Ok(())
}

fn calculate_overall_dimensions(regions: &NbtCompound) -> Result<(i32, i32, i32, i32, i32, i32), Box<dyn std::error::Error>> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut min_z = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut max_z = i32::MIN;

    for (_, region) in regions.inner() {
        if let NbtTag::Compound(region) = region {
            let position = region.get::<_, &NbtCompound>("Position")?;
            let size = region.get::<_, &NbtCompound>("Size")?;

            let x = position.get::<_, i32>("x")?;
            let y = position.get::<_, i32>("y")?;
            let z = position.get::<_, i32>("z")?;
            let width = size.get::<_, i32>("x")?;
            let height = size.get::<_, i32>("y")?;
            let length = size.get::<_, i32>("z")?;

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x + width);
            max_y = max_y.max(y + height);
            max_z = max_z.max(z + length);
        }
    }

    Ok((max_x - min_x, max_y - min_y, max_z - min_z, min_x, min_y, min_z))
}

fn create_unified_palette_and_data(
    regions: &NbtCompound,
    width: i32,
    height: i32,
    length: i32,
    min_x: i32,
    min_y: i32,
    min_z: i32
) -> Result<(NbtCompound, Vec<i8>), Box<dyn std::error::Error>> {
    let mut unified_palette = NbtCompound::new();
    let mut block_data = vec![0i8; (width * height * length) as usize];
    let mut next_palette_id = 0;

    for (_, region) in regions.inner() {
        if let NbtTag::Compound(region) = region {
            let position = region.get::<_, &NbtCompound>("Position")?;
            let size = region.get::<_, &NbtCompound>("Size")?;
            let palette = region.get::<_, &NbtList>("BlockStatePalette")?;
            let block_states = region.get::<_, &NbtTag>("BlockStates")?;

            let region_x = position.get::<_, i32>("x")? - min_x;
            let region_y = position.get::<_, i32>("y")? - min_y;
            let region_z = position.get::<_, i32>("z")? - min_z;
            let region_width = size.get::<_, i32>("x")?;
            let region_height = size.get::<_, i32>("y")?;
            let region_length = size.get::<_, i32>("z")?;

            let bits_per_block = (palette.len() as f64).log2().ceil() as usize;
            let mask = (1u64 << bits_per_block) - 1;

            let mut bit_buffer = 0u64;
            let mut bits_in_buffer = 0;
            let block_states = if let NbtTag::LongArray(block_states) = block_states {
                block_states
            } else {
                return Err("BlockStates is not a LongArray".into());
            };
            let mut block_states_iter = block_states.iter();

            for y in 0..region_height {
                for z in 0..region_length {
                    for x in 0..region_width {
                        while bits_in_buffer < bits_per_block {
                            if let Some(&long_value) = block_states_iter.next() {
                                bit_buffer |= (long_value as u64) << bits_in_buffer;
                                bits_in_buffer += 64;
                            } else {
                                return Err("BlockStates array is too short".into());
                            }
                        }

                        let palette_id = (bit_buffer & mask) as usize;
                        bit_buffer >>= bits_per_block;
                        bits_in_buffer -= bits_per_block;

                        if let NbtTag::Compound(block_state) = &palette[palette_id] {
                            let block_name = block_state.get::<_, &str>("Name")?;
                            let mut full_name = block_name.to_string();

                            if let Ok(Some(NbtTag::Compound(properties))) = block_state.get("Properties") {
                                full_name.push('[');
                                for (key, value) in properties.inner() {
                                    full_name.push_str(&format!("{}={},", key, value));
                                }
                                full_name.pop(); // Remove last comma
                                full_name.push(']');
                            }

                            let unified_id = if let Ok(Some(NbtTag::Int(id))) = unified_palette.get(&full_name) {
                                *id
                            } else {
                                let id = next_palette_id;
                                unified_palette.insert(full_name, NbtTag::Int(id));
                                next_palette_id += 1;
                                id
                            };

                            let index = ((region_x + x) + (region_z + z) * width + (region_y + y) * width * length) as usize;
                            block_data[index] = unified_id as i8;
                        }
                    }
                }
            }
        }
    }

    Ok((unified_palette, block_data))
}

fn collect_entities_and_block_entities(
    regions: &NbtCompound,
    min_x: i32,
    min_y: i32,
    min_z: i32
) -> Result<(Vec<NbtTag>, Vec<NbtTag>), Box<dyn std::error::Error>> {
    let mut entities = Vec::new();
    let mut block_entities = Vec::new();

    for (_, region) in regions.inner() {
        if let NbtTag::Compound(region) = region {
            if let Ok(NbtTag::List(region_entities)) = region.get::<_, &NbtTag>("Entities") {
                for entity in region_entities {
                    if let NbtTag::Compound(mut entity) = entity.clone() {
                        if let Ok(Some(NbtTag::List(mut pos))) = entity.get_mut("Pos") {
                            if let (Some(NbtTag::Double(x)), Some(NbtTag::Double(y)), Some(NbtTag::Double(z))) = (pos.get_mut(0), pos.get_mut(1), pos.get_mut(2)) {
                                *x -= min_x as f64;
                                *y -= min_y as f64;
                                *z -= min_z as f64;
                            }
                        }
                        entities.push(NbtTag::Compound(entity));
                    }
                }
            }

            if let Ok(NbtTag::List(region_block_entities)) = region.get::<_, &NbtTag>("TileEntities") {
                for block_entity in region_block_entities {
                    if let NbtTag::Compound(mut block_entity) = block_entity.clone() {
                        if let Some(NbtTag::IntArray(pos)) = block_entity.get_mut("Pos") {
                            pos[0] -= min_x;
                            pos[1] -= min_y;
                            pos[2] -= min_z;
                        }
                        block_entities.push(NbtTag::Compound(block_entity));
                    }
                }
            }
        }
    }

    Ok((entities, block_entities))
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