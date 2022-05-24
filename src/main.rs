use std::fs;
use std::path::Path;
use std::env;
use goblin::elf::Elf;
use goblin::strtab::Strtab;
use goblin::elf::SectionHeader;
use goblin::container;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir_entries = fs::read_dir(&args[1]).unwrap();
    for dirent in dir_entries {
        let path = dirent.unwrap().path();
        if file_has_syms(&path) {
            println!("Found symbols in {}", path.display());
        }
    }
}

fn file_has_syms(path: &Path) -> bool {
    let tmp = fs::read(path);
    let bytes: Vec<u8>;
    if tmp.is_ok() { bytes = tmp.unwrap(); } else { return false; }
    let header = Elf::parse_header(&bytes).ok();
    match header {
        Some(header) => {
            let elf = Elf::lazy_parse(header).ok();
            match elf {
                Some(mut elf) => {
                    let ctx = container::Ctx {
                        container: container::Container::Little,
                        le: container::Endian::Little,
                    };
                    let sh = SectionHeader::parse(&bytes, header.e_shoff as usize, header.e_shnum as usize, ctx).ok();
                    match sh {
                        Some(section_headers) => {
                            let strtab_idx = header.e_shstrndx as usize;
                            let tmp = get_strtab(&bytes, &section_headers, strtab_idx);
                            if tmp.is_ok() {
                                elf.shdr_strtab = tmp.unwrap();
                            } 
                            else {
                                return false;
                            }

                            elf.section_headers = section_headers;
                            return section_headers_indicate_syms(elf);
                        },
                        _ => return false,
                    }
                }
                None => return false,
            }
        },
        None => return false,
    }
}

fn section_headers_indicate_syms(elf: goblin::elf::Elf) -> bool {
    let mut good_symtab = false; 
    let mut good_strtab = false;
    for header in elf.section_headers.into_iter() {
        let sym_opt = elf.shdr_strtab.get_at(header.sh_name);
        if sym_opt.is_some() && sym_opt.unwrap() == ".symtab" && header.sh_size > 16 {
            good_symtab = true;
        } else if sym_opt.is_some() && sym_opt.unwrap() == ".strtab" && header.sh_size > 16 {
            good_strtab = true;
        }
    }
    
    good_symtab && good_strtab
}

fn get_strtab<'a>(bytes: &'a[u8], section_headers: &[SectionHeader], section_idx: usize) -> Result<Strtab<'a>, goblin::error::Error> {
    if section_idx >= section_headers.len() {
        Ok(Strtab::default())
    } else {
        let shdr = &section_headers[section_idx];
        shdr.check_size(bytes.len())?;
        Strtab::parse(bytes, shdr.sh_offset as usize, shdr.sh_size as usize, 0x0)
    }
}
