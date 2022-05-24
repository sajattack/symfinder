use std::fs;
use std::path::Path;
use std::env;
use goblin::elf::Elf;
use goblin::strtab::Strtab;
use goblin::elf::SectionHeader;
use goblin::container;

fn main() -> Result<(), &'static str> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("Expected a path argument");
    }
    for dirent in fs::read_dir(&args[1]).unwrap() {
        let path = dirent.unwrap().path();
        if file_has_syms(&path).unwrap_or(false) {
            println!("Found symbols in {}", path.display());
        }
    }
    Ok(())
}

fn file_has_syms(path: &Path) -> bool {
    let ctx = container::Ctx {
        container: container::Container::Little,
        le: container::Endian::Little,
    };
    let has_syms = || {
        let bytes = fs::read(path).ok()?;
        let header = Elf::parse_header(&bytes).ok()?;
        let mut elf = Elf::lazy_parse(header).ok()?;
        let section_headers = SectionHeader::parse(&bytes, header.e_shoff as usize, header.e_shnum as usize, ctx).ok()?;
        let shdr_strtab = get_strtab(&bytes, &section_headers, header.e_shstrndx as usize).ok()?;
        elf.shdr_strtab = shdr_strtab;
        elf.section_headers = section_headers;
        Some(section_headers_indicate_syms(elf))
    };
    has_syms().unwrap_or(false)
}

fn section_headers_indicate_syms(elf: goblin::elf::Elf) -> bool {
    let mut good_symtab = false; 
    let mut good_strtab = false;
    for header in elf.section_headers.into_iter() {
        let sym_opt = elf.shdr_strtab.get_at(header.sh_name);
        match sym_opt {
            Some(".symtab") if header.sh_size > 16 => good_symtab = true,
            Some(".strtab") if header.sh_size > 16 => good_strtab = true,
            _ => continue
        }
    }
    good_symtab && good_strtab
}

fn get_strtab<'a>(bytes: &'a[u8], section_headers: &[SectionHeader], section_idx: usize) -> Result<Strtab<'a>, goblin::error::Error> {
    if section_idx < section_headers.len() {
        let shdr = &section_headers[section_idx];
        shdr.check_size(bytes.len())?;
        return Strtab::parse(bytes, shdr.sh_offset as usize, shdr.sh_size as usize, 0x0);
    }
    Ok(Strtab::default())
}
