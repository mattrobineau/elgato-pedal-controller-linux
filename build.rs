use regex::Regex;
use std::{env, fs, path::Path, path::PathBuf};
use std::fs::File;
use std::io;
use std::io::prelude::*;

fn main() {
    let target_deps_path = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Could not find CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("deps");
    
    let enigo_file_path = find_enigo_file(&target_deps_path).expect("Enigo file not found");
    let keycodes_path = extract_keycodes_path(&enigo_file_path).expect("keycodes.rs path not found");
    

    if let Some((enum_name, variants)) = extract_enum_name_and_variants(keycodes_path) {
        // Specify the output file path
        let output_file_path = Path::new("./src/key_definitions.rs");
        
        // Write the enum declaration to the file
        let _ = write_enum_declaration_to_file(&enum_name, &variants, output_file_path);
        
        println!("Enum written to {:?}", output_file_path);
    } else {
        println!("Enum name and variants could not be extracted.");
    }

}

fn find_enigo_file(dir_path: &Path) -> Option<PathBuf> {
  let regex = Regex::new(r"^enigo").ok()?;
  if dir_path.is_dir() {

        for entry in fs::read_dir(dir_path).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if regex.is_match(file_name) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn extract_keycodes_path(enigo_file_path: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(enigo_file_path).ok()?;
    let re = Regex::new(r"keycodes\.rs").expect("Could not find enigo's keycodes.rs file");
    let mut file_path = PathBuf::new();
    for line in content.lines() {
        if re.is_match(line) {
            file_path = PathBuf::from(line.trim().trim_end_matches(':'));
        }
    }
    return Some(file_path)
}


fn extract_enum_name_and_variants(keycodes_file_path: PathBuf) -> Option<(String, Vec<String>)> {
    let content: String = fs::read_to_string(&keycodes_file_path).expect("Could not read file");

    let enum_name_re = Regex::new(r"match\s+(\w+)\s+\{").expect("Failed to compile enum name regex");
    let variant_re = Regex::new(r"Key::(\w+[\w\(\),]*)").expect("Failed to compile variant regex");

    let enum_name = enum_name_re.captures(&content)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_owned()));

        // Early return if no enum name is found
    let enum_name = match enum_name {
        Some(name) => name,
        None => return None,
    };

    let enum_block_start = enum_name_re.find(&content)?.start();
    let enum_block_end = content[enum_block_start..].find('}').map(|end| enum_block_start + end + 1).unwrap_or_else(|| content.len());

    let variants = variant_re.captures_iter(&content[enum_block_start..enum_block_end])
        .filter_map(|cap| {
            let variant = cap.get(1)?.as_str().trim().to_owned();
            if !variant.contains("(") {
                Some(variant)
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    Some((enum_name, variants))
}

fn write_enum_declaration_to_file(enum_name: &str, variants: &[String], file_path: &Path) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "// This file was automatically created at build time.")?;
    writeln!(file, "//   It extracts the key definitions of the enigo crate that are marked for linux.")?;
    writeln!(file, "//   It excludes two key definitions, `unicode` (to enter any character), and `other`.")?;
    writeln!(file, "")?;
    writeln!(file, "use lazy_static::lazy_static;")?;
    writeln!(file, "use std::collections::HashMap;")?;
    writeln!(file, "use enigo::*;")?;
    writeln!(file, "lazy_static! {{")?;
    writeln!(file, "    pub static ref {}_DEFINITIONS: HashMap<&'static str, Key> = {{", enum_name.to_uppercase())?;
    writeln!(file, "        let mut m = HashMap::new();")?;
    for variant in variants {
        writeln!(file, "        m.insert(\"{}\", Key::{});", variant, variant)?;
    }
    writeln!(file, "        m")?;
    writeln!(file, "    }};")?;
    writeln!(file, "}}")?;

    Ok(())
}