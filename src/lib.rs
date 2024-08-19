use std::io::Cursor;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod converters;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug)]
pub enum SchematicFormat {
    Litematic,
    Schematic,
    Schem,
}

pub struct SchematicConverter;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl SchematicConverter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        SchematicConverter
    }

    pub fn convert(&self, input: &[u8], from: SchematicFormat, to: SchematicFormat) -> Result<Vec<u8>, JsValue> {
        self.convert_internal(input, from, to)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SchematicConverter {
    pub fn new() -> Self {
        SchematicConverter
    }

    pub fn convert(&self, input: &[u8], from: SchematicFormat, to: SchematicFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.convert_internal(input, from, to)
    }
}

impl SchematicConverter {
    fn convert_internal(&self, input: &[u8], from: SchematicFormat, to: SchematicFormat) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut output = Vec::new();
        let result = match (from, to) {
            (SchematicFormat::Litematic, SchematicFormat::Schematic) => {
                converters::litematic_to_schematic(Cursor::new(input), &mut output)
            }
            (SchematicFormat::Schematic, SchematicFormat::Schem) => {
                converters::schematic_to_schem(Cursor::new(input), &mut output)
            }
            (SchematicFormat::Schematic, SchematicFormat::Litematic) => {
                converters::schematic_to_litematic(Cursor::new(input), &mut output)
            }
            _ => Err("Unsupported conversion path".into()),
        };
        result.map(|_| output)
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

pub use converters::*;