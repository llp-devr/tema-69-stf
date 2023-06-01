use encoding_rs::*;
use encoding_rs_io::DecodeReaderBytesBuilder;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

pub(crate) fn summarize(path: PathBuf) -> HashMap<(String, String, String), f64> {
    let mut cnpj: String = String::new();
    let mut summary: HashMap<(String, String, String), f64> = HashMap::new();

    let file: File = File::open(&path).unwrap();

    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252)) // ISO-8859-1 encoding
        .build(file);

    let reader = BufReader::new(decoder);

    let mut cod_mod: String = String::new();

    for line in reader.lines() {
        let l = line.unwrap();
        let r: Vec<&str> = l.split('|').collect();

        if let Some(reg) = r.get(1) {
            match *reg {
                "0000" => cnpj = r.get(7).unwrap().to_string(),

                "C100" => cod_mod = r.get(5).unwrap().to_string(),

                "C190" => {
                    let cfop: String = r.get(3).unwrap().to_string();
                    let vl_icms: f64 =
                        f64::from_str(&r.get(7).unwrap().replace(',', ".")).unwrap();

                    let key = (cnpj.clone(), cod_mod.clone(), cfop);
                    let value = summary.entry(key).or_insert(0_f64);
                    *value += vl_icms;
                }

                "C500" => cod_mod = r.get(5).unwrap().to_string(),

                "C590" => {
                    let cfop: String = r.get(3).unwrap().to_string();
                    let vl_icms: f64 = r.get(7).unwrap().parse().unwrap();

                    let key = (cnpj.clone(), cod_mod.clone(), cfop);
                    let value = summary.entry(key).or_insert(0_f64);
                    *value += vl_icms;
                }

                "C320" | "C390" | "C490" | "C690" | "C790" | "C850" | "C890" | "D190" | "D300"
                | "D390" | "D410" | "D590" | "D690" | "D696" => {
                    todo!("Registro {} não implantado", reg)
                }

                _ => {}
            }
        }
    }

    summary
}
