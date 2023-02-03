use seq_geom_parser::{FragmentGeomDesc, PiscemGeomDesc, SalmonSeparateGeomDesc};

#[test]
fn test_parse_piscem() {
    let arg = "1{b[16]u[12]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            let piscem_desc =
                PiscemGeomDesc::from_geom_pieces(&frag_desc.read1_desc, &frag_desc.read2_desc);

            assert_eq!(
                piscem_desc,
                PiscemGeomDesc {
                    read1_desc: "{b[16]u[12]x:}".to_string(),
                    read2_desc: "{r:}".to_string()
                }
            );
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

#[test]
fn test_salmon_piscem() {
    let arg = "1{b[16]u[12]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            let salmon_desc = SalmonSeparateGeomDesc::from_geom_pieces(
                &frag_desc.read1_desc,
                &frag_desc.read2_desc,
            );

            assert_eq!(
                salmon_desc,
                SalmonSeparateGeomDesc {
                    barcode_desc: "1[1-16]".to_string(),
                    umi_desc: "1[17-28]".to_string(),
                    read_desc: "2[1-end]".to_string()
                }
            );
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

#[test]
fn test_fail_on_bad_geo() {
    let arg = "1{b[16]v[3]u[12]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(_frag_desc) => {
            panic!("this should not be parsed {}", arg);
        }
        Err(_e) => {}
    };
}
