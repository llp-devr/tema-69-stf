use encoding_rs::*;
use encoding_rs_io::DecodeReaderBytesBuilder;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

struct C175Value {
    vl_icms: f64,
    vl_opr_cfop5102: f64,
    vl_opr_cfop5102_cst01: f64,
}

struct C180Value {
    vl_opr_cfop5102: f64,
    vl_opr_cfop5102_cst01: f64,
}

pub struct Apuracao {
    pub(crate) vl_rec_brt: f64,
    pub(crate) vl_bc_cont: f64,
    pub(crate) aliq_cont: f64,
}

fn to_f64(input: &Option<&String>) -> f64 {
    let replaced = input.unwrap().replace(',', ".");
    replaced.parse::<f64>().unwrap_or(0_f64)
}

pub(crate) fn summarize(path: PathBuf, efd_icms_ipi: HashMap<(String, String, String), f64>) -> (HashMap<String, f64>, Vec<Apuracao>, Vec<Apuracao>) {
    let mut summary: HashMap<String, f64> = HashMap::new();
    let mut m210: Vec<Apuracao> = Vec::new();
    let mut m610: Vec<Apuracao> = Vec::new();

    println!("{}", &path.display());

    let file: File = File::open(&path).unwrap();

    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252)) // ISO-8859-1 encoding
        .build(file);

    let reader = BufReader::new(decoder);

    let mut c010_cnpj: String = String::new();
    let mut c010_ind_escri: String = String::new();

    let mut c100_ind_oper: String = String::new();
    let mut c100_cod_mod: String = String::new();
    let mut c100_chv_nfe: String = String::new();
    let mut c100_vl_icms: f64 = f64::from(0);

    let mut c175_cache: HashMap<String, C175Value> = HashMap::new();

    let mut c180_cod_mod: String = String::new();
    let mut c180_cache: HashMap<(String, String, String), C180Value> = HashMap::new();

    for line in reader.lines() {
        let l = line.unwrap();
        let r: Vec<String> = l.split('|').map(|s| s.to_string()).collect();

        if let Some(reg) = r.get(1) {
            match reg.as_str() {
                "C010" => {
                    c010_cnpj = r.get(2).unwrap().to_string();
                    c010_ind_escri = r.get(3).unwrap().to_string();
                }

                "C100" => {
                    c100_ind_oper = r.get(2).unwrap().to_string();
                    c100_cod_mod = r.get(5).unwrap().to_string();
                    c100_chv_nfe = r.get(9).unwrap().to_string();
                    c100_vl_icms = to_f64(&r.get(22));
                }

                "C170" => {
                    if c100_ind_oper != "1" {
                        continue;
                    }

                    if (c100_cod_mod != "55" || c010_ind_escri != "1") && *r.get(25).unwrap() == "01" {
                        let vl_icms = to_f64(&r.get(15));

                        let value = summary.entry(reg.to_string()).or_insert(0_f64);
                        *value += vl_icms;
                    }
                }

                "C175" => {
                    if *r.get(2).unwrap() == "5102" {
                        let value = c175_cache.entry(c100_chv_nfe.clone()).or_insert(C175Value {
                            vl_icms: c100_vl_icms,
                            vl_opr_cfop5102: 0_f64,
                            vl_opr_cfop5102_cst01: 0_f64,
                        });

                        let vl_opr: f64 = to_f64(&r.get(3));

                        value.vl_opr_cfop5102 += vl_opr;

                        if *r.get(5).unwrap() == "01" {
                            value.vl_opr_cfop5102_cst01 += vl_opr;
                        }
                    }
                }

                "C180" => c180_cod_mod = r.get(2).unwrap().clone(),

                "C181" => {
                    if c010_ind_escri != "2" || c180_cod_mod == "65" {
                        println!("{:?}", r);

                        if *r.get(3).unwrap() == "5102" {
                            let vl_opr: f64 = to_f64(&r.get(4));
                            let cfop: String = r.get(3).unwrap().to_string();

                            let key = (c010_cnpj.clone(), c180_cod_mod.clone(), cfop);
                            let value = c180_cache.entry(key).or_insert(C180Value {
                                vl_opr_cfop5102: 0_f64,
                                vl_opr_cfop5102_cst01: 0_f64,
                            });

                            value.vl_opr_cfop5102 += vl_opr;

                            if *r.get(2).unwrap() == "01" {
                                value.vl_opr_cfop5102_cst01 += vl_opr;
                            }
                        } else {
                            todo!("Invalid {}", r.get(3).unwrap());
                        }
                    }
                }

                "C381" | "C385" | "C481" | "C485" | "C491" | "C495" | "C601" | "C605"
                | "C870" | "D201" | "D205" | "D300" | "D350" | "D601" | "D605" | "F100"
                | "F500" | "F550" => {
                    todo!("Registro {} não implantado", reg)
                }

                "M210" => {
                    if *r.get(2).unwrap() == "01" || *r.get(2).unwrap() == "51" {
                        if r.len() == 15 {
                            m210.push(Apuracao {
                                vl_rec_brt: to_f64(&r.get(3)),
                                vl_bc_cont: to_f64(&r.get(4)),
                                aliq_cont: to_f64(&r.get(5)),
                            })
                        } else {
                            m210.push(Apuracao {
                                vl_rec_brt: to_f64(&r.get(3)),
                                vl_bc_cont: to_f64(&r.get(7)),
                                aliq_cont: to_f64(&r.get(8)),
                            })
                        }
                    }
                }

                "M610" => {
                    if *r.get(2).unwrap() == "01" || *r.get(2).unwrap() == "51" {
                        if r.len() == 15 {
                            m610.push(Apuracao {
                                vl_rec_brt: to_f64(&r.get(3)),
                                vl_bc_cont: to_f64(&r.get(4)),
                                aliq_cont: to_f64(&r.get(5)),
                            })
                        } else {
                            m610.push(Apuracao {
                                vl_rec_brt: to_f64(&r.get(3)),
                                vl_bc_cont: to_f64(&r.get(7)),
                                aliq_cont: to_f64(&r.get(8)),
                            })
                        }
                    }
                }

                _ => {}
            }
        }
    }

    for (_key, value) in c175_cache.iter() {
        let vl_icms: f64 = value.vl_icms / value.vl_opr_cfop5102 * value.vl_opr_cfop5102_cst01;
        let value = summary.entry("C175".to_string()).or_insert(0_f64);
        *value += vl_icms;
    }

    if c180_cache.is_empty() {} else {
        for (key, value) in c180_cache {
            let vl_icms: f64 = *efd_icms_ipi.get(&key.clone()).unwrap();

            let vl_icms_prop: f64 = vl_icms / value.vl_opr_cfop5102 * value.vl_opr_cfop5102_cst01;

            let value = summary.entry("C180".to_string()).or_insert(0_f64);
            *value += vl_icms_prop;
        }
    }

    (summary, m210, m610)
}
