use fltk::{app::*, button::*, dialog::*, enums::Font, input::*, prelude::*, window::*};

use serde::Serialize;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::rc::Rc;
use fltk::output::MultilineOutput;

mod efd_contribuicoes;
mod efd_icms_ipi;

struct Console {
    display: MultilineOutput,
    lines: Vec<String>,
}

impl Console {
    fn new() -> Self {
        let mut display = MultilineOutput::new(10, 10, 1480, 730, "");
        display.set_text_font(Font::Courier);
        display.set_text_size(12);

        Self {
            display,
            lines: Vec::new(),
        }
    }

    fn add_line(&mut self, line: String) {
        self.lines.push(line);
        self.display.set_value(&self.get_text());
    }

    fn clear(&mut self) {
        self.lines = Vec::<String>::new();
        self.display.set_value(&self.get_text());
    }

    fn get_text(&self) -> String {
        self.lines.join("\n")
    }
}

#[derive(Serialize)]
struct Sped {
    #[serde(rename = "EFD Contribuições")]
    efd_contribuicoes: Option<PathBuf>,

    #[serde(rename = "EFD ICMS/IPI")]
    efd_icms_ipi: Vec<PathBuf>,

    #[serde(rename = "Competência")]
    competencia: Option<String>,
}

impl Sped {
    fn new(filenames: Vec<PathBuf>) -> Self {
        let mut efd_contribuicoes: Option<PathBuf> = None;
        let mut efd_icms_ipi: Vec<PathBuf> = Vec::new();
        let mut competencia: Option<String> = None;

        for path in filenames {
            if let Ok(file) = File::open(&path) {
                let reader = BufReader::new(file);

                if let Some(Ok(first_line)) = reader.lines().next() {
                    let data: Vec<&str> = first_line.split('|').collect();

                    if competencia.is_none() {
                        // Check CNPJ at index 9 for EFD Contribuições
                        if is_cnpj_valid(data.get(9).unwrap()) {
                            efd_contribuicoes = Some(path.clone());
                            competencia = Some(data.get(7).unwrap().to_string())
                        }

                        // Check CNPJ at index 7 for EFD ICMS/IPI
                        if is_cnpj_valid(data.get(7).unwrap()) {
                            efd_icms_ipi.push(path.clone());
                            competencia = Some(data.get(5).unwrap().to_string())
                        }
                    } else {
                        // Check CNPJ at index 9 for EFD Contribuições
                        if is_cnpj_valid(data.get(9).unwrap())
                            && competencia == Some(data.get(7).unwrap().to_string())
                        {
                            efd_contribuicoes = Some(path.clone());
                        }

                        // Check CNPJ at index 7 for EFD ICMS/IPI
                        if is_cnpj_valid(data.get(7).unwrap())
                            && competencia == Some(data.get(5).unwrap().to_string())
                        {
                            efd_icms_ipi.push(path.clone());
                        }
                    }
                }
            }
        }

        Self {
            efd_contribuicoes,
            efd_icms_ipi,
            competencia,
        }
    }
}

impl fmt::Display for Sped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Arquivos em análise:")?;
        if let Some(competencia) = &self.competencia {
            writeln!(f, "- Competência: {}", competencia)?;
        } else {
            writeln!(f, "- Competência: None")?;
        }
        if let Some(efd_contribuicoes) = &self.efd_contribuicoes {
            let filename = efd_contribuicoes
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            writeln!(f, "- EFD Contribuições: {}", filename)?;
        } else {
            writeln!(f, "- EFD Contribuições: None")?;
        }
        write!(f, "- EFD ICMS/IPI: ")?;
        if self.efd_icms_ipi.is_empty() {
            writeln!(f, "None")?;
        } else {
            writeln!(f)?;
            for path in &self.efd_icms_ipi {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                writeln!(f, "  - {}", filename)?;
            }
        }
        Ok(())
    }
}

fn is_cnpj_valid(cnpj: &str) -> bool {
    let digits: Vec<u32> = cnpj.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() != 14 || digits.windows(2).all(|pair| pair[0] == pair[1]) {
        return false;
    }

    let first_check_digit = {
        let weights = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

        let sum = digits
            .iter()
            .zip(&weights)
            .map(|(&d, &w)| d * w)
            .sum::<u32>();

        let remainder = sum % 11;

        if remainder < 2 {
            0
        } else {
            11 - remainder
        }
    };

    let second_check_digit = {
        let weights = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

        let sum = digits
            .iter()
            .zip(&weights)
            .map(|(&d, &w)| d * w)
            .sum::<u32>();

        let remainder = sum % 11;

        if remainder < 2 {
            0
        } else {
            11 - remainder
        }
    };

    digits[12] == first_check_digit && digits[13] == second_check_digit
}

fn process_files(console: Rc<RefCell<Console>>, files: Vec<PathBuf>) {
    console.borrow_mut().clear();

    let files = Sped::new(files);

    console.borrow_mut().add_line(files.to_string());

    let mut efd_icms_ipi: HashMap<(String, String, String), f64> = HashMap::new();

    // Iterate over EFD ICMS/IPI files and append summaries to efd_icms_ipi
    for path in files.efd_icms_ipi {
        let summary = efd_icms_ipi::summarize(path);
        efd_icms_ipi.extend(summary);
    }

    console
        .borrow_mut()
        .add_line("Arquivos EFD ICMS/IPI".to_string());
    if efd_icms_ipi.is_empty() {
        console
            .borrow_mut()
            .add_line("- Não foram apresentados arquivos EFD ICMS/IPI".to_string());
    } else {
        for (key, value) in efd_icms_ipi.iter() {
            if *value > 0_f64 {
                console.borrow_mut().add_line(format!(
                    "- FILIAL: {}; COD_MOD: {}; CFOP: {}; VL_ICMS: {:.2}",
                    key.0, key.1, key.2, value
                ));
            }
        }
    }
    console.borrow_mut().add_line("\n".to_string());

    let (efd_contribuicoes, m210, m610) = efd_contribuicoes::summarize(files.efd_contribuicoes.unwrap(), efd_icms_ipi.clone());

    console
        .borrow_mut()
        .add_line("Arquivos EFD Contribuições".to_string());
    if efd_contribuicoes.is_empty() {
        console
            .borrow_mut()
            .add_line("- Não foram apresentados arquivos EFD Contribuições".to_string());
    } else {
        for (key, value) in efd_contribuicoes.iter() {
            if *value > 0_f64 {
                console.borrow_mut().add_line(format!(
                    "- REG: {}; VL_ICMS: {:.2}",
                    key, value
                ));
            }
        }
    }
    console.borrow_mut().add_line("\n".to_string());

    let vl_icms: f64 = efd_contribuicoes.values().fold(0_f64, |acc, &value| acc + value);
    let vl_rec_brt: f64 = m210.iter().fold(0_f64, |acc, apuracao| acc + apuracao.vl_rec_brt);

    let mut pis: f64 = 0_f64;
    let mut cofins: f64 = 0_f64;

    console.borrow_mut().add_line("Analisando registro M210...".to_string());
    for i in m210 {
        let proporcao = i.vl_rec_brt / vl_rec_brt;
        console.borrow_mut().add_line(format!("- Base de cálculo original: {:.2}", i.vl_bc_cont));
        let icms = vl_icms * proporcao;
        console.borrow_mut().add_line(format!("  ICMS a ser excluído: {:.2}", icms));
        let economia = (icms * i.aliq_cont) / 100_f64;
        pis += economia;
        console.borrow_mut().add_line(format!("  Economia tributária (PIS): {:.2}", economia));
        console.borrow_mut().add_line("\n".to_string());
    }

    console.borrow_mut().add_line("Analisando registro M610...".to_string());
    for i in m610 {
        let proporcao = i.vl_rec_brt / vl_rec_brt;
        console.borrow_mut().add_line(format!("- Base de cálculo original: {:.2}", i.vl_bc_cont));
        let icms = vl_icms * proporcao;
        console.borrow_mut().add_line(format!("  ICMS a ser excluído: {:.2}", icms));
        let economia = (icms * i.aliq_cont) / 100_f64;
        cofins += economia;
        console.borrow_mut().add_line(format!("  Economia tributária (COFINS): {:.2}", economia));
        console.borrow_mut().add_line("\n".to_string());
    }

    console.borrow_mut().add_line("\n".to_string());
    console.borrow_mut().add_line("Economia tributária total...".to_string());
    console.borrow_mut().add_line(format!("- PIS: {:.2}", pis));
    console.borrow_mut().add_line(format!("- COFINS: {:.2}", cofins));
}

fn main() {
    let app = App::default();
    let mut wind = Window::new(100, 100, 1500, 800, "Tema 69 STF");

    let console = Rc::new(RefCell::new(Console::new()));

    let mut upload_button = Button::new(1290, 750, 200, 40, "Adicionar Arquivo");
    upload_button.set_callback(move |_| {
        let mut dialog = FileDialog::new(FileDialogType::BrowseMultiFile);
        dialog.show();

        process_files(console.clone(), dialog.filenames());
    });

    wind.end();
    wind.show();

    app.run().unwrap();
}
