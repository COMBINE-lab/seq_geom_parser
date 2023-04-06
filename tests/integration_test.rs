use seq_geom_parser::{FragmentGeomDesc, PiscemGeomDesc, SalmonSeparateGeomDesc};

/// Parsing a simple format (10xV3 in this case) should work.
/// We check this by ensuring that the format description makes the
/// round trip through parsing and back through printing.
#[test]
fn test_parse_format_simple() {
    let arg = "1{b[16]u[12]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            assert_eq!(arg, format!("{}", frag_desc));
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

/// Parsing a complex format (sciseqv3 in this case) should work.
/// We check this by ensuring that the format description makes the
/// round trip through parsing and back through printing.
#[test]
fn test_parse_format_complex() {
    let arg = "1{b[9-10]f[ACCGT]u[12]b[10]}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            assert_eq!(arg, format!("{}", frag_desc));
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

/// Parsing a complex format with unbounded sequence before an anchor 
/// (10x crispr feature barcoding) should work.
/// We check this by ensuring that the format description makes the
/// round trip through parsing and back through printing.
#[test]
fn test_parse_format_complex_crispr() {
    let arg = "1{b[16]u[12]}2{x:r[20]f[GTTTAAGAGCTAAGCTGGAA]x:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            assert_eq!(arg, format!("{}", frag_desc));
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

/// Parsing a simple format into a `PiscemGeomDesc` should work.
/// We check this by ensuring that the format description makes the
/// round trip through parsing and back through printing.
#[test]
fn test_parse_piscem_simple() {
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
fn test_parse_piscem_complex() {
    let arg = "1{b[16-18]f[ACG]u[12]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(frag_desc) => {
            let piscem_desc =
                PiscemGeomDesc::from_geom_pieces(&frag_desc.read1_desc, &frag_desc.read2_desc);

            assert_eq!(
                piscem_desc,
                PiscemGeomDesc {
                    read1_desc: "{b[16-18]f[ACG]u[12]x:}".to_string(),
                    read2_desc: "{r:}".to_string()
                }
            );
        }
        Err(e) => {
            panic!("Failed to parse geometry {}", e);
        }
    };
}

/// Parsing a simple format into a `PiscemGeomDesc` should work.
/// We check this by ensuring that the format description makes the
/// round trip through parsing  and ensure that it parsed as what
/// we expect.
#[test]
fn test_salmon_simple() {
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

/// Parsing an invalid geometry description string should lead to
/// an `Err` returned from the parser.
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

/// Parsing a bad geometry (one that has valid tokens but doesn't follow
/// the actual grammar) description string should lead to an `Err` returned
/// from the parser.
#[test]
fn test_fail_on_ambig_geo() {
    let arg = "1{b[16]u[12-13]x:}2{r:}";
    match FragmentGeomDesc::try_from(arg) {
        Ok(_frag_desc) => {
            panic!("this should not be parsed {}", arg);
        }
        Err(_e) => {}
    };
}

/// Parsing a bad geometry (a proper geometry followed by nonsense)
/// description string should lead to an `Err` returned from the parser.
#[test]
fn test_fail_on_superfluous_input() {
    let arg = "1{b[16]u[12]x:}2{r:}_flargbarg";
    match FragmentGeomDesc::try_from(arg) {
        Ok(_frag_desc) => {
            panic!("this should not be parsed {}", arg);
        }
        Err(_e) => {}
    };
}
