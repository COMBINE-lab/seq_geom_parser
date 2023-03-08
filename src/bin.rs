extern crate pest;

use anyhow::{bail, Result};
use seq_geom_parser::{AppendToCmdArgs, FragmentGeomDesc, PiscemGeomDesc, SalmonSeparateGeomDesc};

fn main() -> Result<()> {
    let arg_owned = std::env::args().nth(1).unwrap();
    let arg: &str = &arg_owned;
    println!("arg = {}", arg);

    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            println!("parsed geometry : {:#?}", &frag_desc);

            if frag_desc.is_simple_geometry() {
                let piscem_desc =
                    PiscemGeomDesc::from_geom_pieces(&frag_desc.read1_desc, &frag_desc.read2_desc);

                let salmon_desc = SalmonSeparateGeomDesc::from_geom_pieces(
                    &frag_desc.read1_desc,
                    &frag_desc.read2_desc,
                );

                println!(
                    "salmon desc: {:?}\npiscem_desc: {:?}",
                    salmon_desc, piscem_desc
                );

                let mut cmd_piscem = std::process::Command::new("piscem");
                piscem_desc.append(&mut cmd_piscem);
                println!("piscem cmd : {:?}", cmd_piscem);

                let mut cmd_salmon = std::process::Command::new("salmon");
                salmon_desc.append(&mut cmd_salmon);
                println!("salmon cmd : {:?}", cmd_salmon);
            }
            /*
            let piscem_desc =
                PiscemGeomDesc::from_geom_pieces(&frag_desc.read1_desc, &frag_desc.read2_desc);
            let salmon_desc = SalmonSeparateGeomDesc::from_geom_pieces(
                &frag_desc.read1_desc,
                &frag_desc.read2_desc,
            );

            println!(
                "salmon desc: {:?}\npiscem_desc: {:?}",
                salmon_desc, piscem_desc
            );

            let mut cmd_piscem = std::process::Command::new("piscem");
            piscem_desc.append(&mut cmd_piscem);
            println!("piscem cmd : {:?}", cmd_piscem);

            let mut cmd_salmon = std::process::Command::new("salmon");
            salmon_desc.append(&mut cmd_salmon);
            println!("salmon cmd : {:?}", cmd_salmon);
            */
        }
        Err(e) => {
            bail!(e);
        }
    };
    Ok(())
}
