extern crate pest;
#[macro_use]
extern crate pest_derive;

use anyhow::{anyhow, Result};
use pest::Parser;

use std::convert::TryFrom;
use std::fmt;

#[derive(Parser)]
#[grammar = "grammar/frag_geom.pest"] // relative to src
pub struct FragGeomParser;

/// The types of lengths that a piece of
/// geometry can have.
#[derive(Debug, Copy, Clone)]
pub enum GeomLen {
    FixedLen(u32),
    LenRange(u32, u32),
    Unbounded,
}

/// Represents the sequence held by a fixed
/// sequence anchor.
#[derive(Debug, Clone)]
pub enum NucStr {
    Seq(String),
}

/// The pieces of geometry (types) we
/// currently support.
#[derive(Debug, Clone)]
pub enum GeomPiece {
    Barcode(GeomLen),
    Umi(GeomLen),
    Discard(GeomLen),
    ReadSeq(GeomLen),
    Fixed(NucStr),
}

impl fmt::Display for GeomPiece {
    /// Formats and returns the canonical string representation of each type of
    /// `GeomPiece`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            GeomPiece::Umi(GeomLen::Unbounded) => write!(f, "u:"),
            GeomPiece::Barcode(GeomLen::Unbounded) => write!(f, "b:"),
            GeomPiece::ReadSeq(GeomLen::Unbounded) => write!(f, "r:"),
            GeomPiece::Discard(GeomLen::Unbounded) => write!(f, "x:"),
            GeomPiece::Umi(GeomLen::FixedLen(x)) => write!(f, "u[{}]", x),
            GeomPiece::Barcode(GeomLen::FixedLen(x)) => write!(f, "b[{}]", x),
            GeomPiece::ReadSeq(GeomLen::FixedLen(x)) => write!(f, "r[{}]", x),
            GeomPiece::Discard(GeomLen::FixedLen(x)) => write!(f, "x[{}]", x),
            GeomPiece::Umi(GeomLen::LenRange(l, h)) => write!(f, "u[{}-{}]", l, h),
            GeomPiece::Barcode(GeomLen::LenRange(l, h)) => write!(f, "b[{}-{}]", l, h),
            GeomPiece::ReadSeq(GeomLen::LenRange(l, h)) => write!(f, "r[{}-{}]", l, h),
            GeomPiece::Discard(GeomLen::LenRange(l, h)) => write!(f, "x[{}-{}]", l, h),
            GeomPiece::Fixed(NucStr::Seq(s)) => write!(f, "f[{}]", s),
        }
    }
}

impl GeomPiece {
    /// This method returns true if the current GeomPiece has a fixed length
    /// (either FixedLen or a Fixed(NucStr)), and false otherwise.
    pub fn is_fixed_len(&self) -> bool {
        matches!(
            self,
            GeomPiece::Umi(GeomLen::FixedLen(_))
                | GeomPiece::Barcode(GeomLen::FixedLen(_))
                | GeomPiece::ReadSeq(GeomLen::FixedLen(_))
                | GeomPiece::Discard(GeomLen::FixedLen(_))
                | GeomPiece::Fixed(NucStr::Seq(_))
        )
    }

    /// This method returns true if the current GeomPiece has a bounded length
    /// (either Bounded, BoundedRange, or a Fixed(NucStr)), and false otherwise.
    pub fn is_bounded(&self) -> bool {
        !matches!(
            self,
            GeomPiece::Umi(GeomLen::Unbounded)
                | GeomPiece::Barcode(GeomLen::Unbounded)
                | GeomPiece::ReadSeq(GeomLen::Unbounded)
                | GeomPiece::Discard(GeomLen::Unbounded)
        )
    }

    /// This method returns true if the current GeomPiece is "complex"
    /// (either BoundedRange, or a Fixed(NucStr)), and false otherwise.
    pub fn is_complex(&self) -> bool {
        matches!(
            self,
            GeomPiece::Fixed(NucStr::Seq(_))
                | GeomPiece::Umi(GeomLen::LenRange(_, _))
                | GeomPiece::Barcode(GeomLen::LenRange(_, _))
                | GeomPiece::ReadSeq(GeomLen::LenRange(_, _))
                | GeomPiece::Discard(GeomLen::LenRange(_, _))
        )
    }
}

// functions for parsing the different types of geometry elements

/// Parses a string "x" (assumed to be parsable as a `u32`) into an
/// integer x and returns x.
fn parse_fixed_len_as_u32(r: &mut pest::iterators::Pairs<Rule>) -> u32 {
    let rn = r.next().unwrap();
    match rn.as_rule() {
        Rule::single_len => {
            return rn.as_str().parse::<u32>().unwrap();
        }
        r => unimplemented!("Expected rule 'single_len', but found {:?}", r),
    }
}

/// Parses a string "x" (assumed to be parsable as a `u32`) into an
/// integer x and returns `GeomLen::FixedLen(x)`.
fn parse_fixed_len(r: &mut pest::iterators::Pairs<Rule>) -> GeomLen {
    GeomLen::FixedLen(parse_fixed_len_as_u32(r))
}

/// Parses a range of the format, "l-h" (where "l" and "h" assumed to be parsable as a `u32`)
/// and returns `GeomLen::LenRange(l, h)`.
fn parse_ranged_len(r: &mut pest::iterators::Pairs<Rule>) -> GeomLen {
    let rn = r.next().unwrap();
    match rn.as_rule() {
        Rule::len_range => {
            let mut ri = rn.into_inner();
            let l = parse_fixed_len_as_u32(&mut ri);
            let h = parse_fixed_len_as_u32(&mut ri);
            return GeomLen::LenRange(l, h);
        }
        r => unimplemented!("expected rule 'len_range' but found {:?}", r),
    }
}

/// Parses a fixed nucleotide sequence s (matching "[ACGT]+") and returns
/// `NucStr::Seq(s)`.
fn parse_fixed_seq(r: &mut pest::iterators::Pairs<Rule>) -> NucStr {
    let rn = r.next().unwrap();
    match rn.as_rule() {
        Rule::nucstr => {
            let seq_str = rn.as_str();
            return NucStr::Seq(seq_str.to_owned());
        }
        r => unimplemented!("expected rule 'nucstr' but found {:?}", r),
    }
}

/// Parses a `GeomPiece` that represents a "ranged segment", that is a
/// barcode, umi, read string, or discard segment having a ranged length.
fn parse_ranged_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::ranged_umi_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            return GeomPiece::Umi(gl);
        }
        Rule::ranged_barcode_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            return GeomPiece::Barcode(gl);
        }
        Rule::ranged_discard_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            return GeomPiece::Discard(gl);
        }
        Rule::ranged_read_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            return GeomPiece::ReadSeq(gl);
        }
        _ => unimplemented!(),
    }
}

/// Parses a `GeomPiece` that represents a "ranged segment", that is a
/// barcode, umi, read string, discard segment, or fixed seq segment having a fixed length.
fn parse_fixed_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::fixed_umi_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            return GeomPiece::Umi(gl);
        }
        Rule::fixed_barcode_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            return GeomPiece::Barcode(gl);
        }
        Rule::fixed_discard_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            return GeomPiece::Discard(gl);
        }
        Rule::fixed_read_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            return GeomPiece::ReadSeq(gl);
        }
        Rule::fixed_seq_segment => {
            let fseq = parse_fixed_seq(&mut r.into_inner());
            return GeomPiece::Fixed(fseq);
        }
        _ => unimplemented!(),
    };
}

/// Parses a `GeomPiece` that represents an "unbounded segment", that is a
/// barcode, umi, read string, or discard segment that is not of fixed length
/// (i.e. that has length >=1).
fn parse_unbounded_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::unbounded_umi_segment => GeomPiece::Umi(GeomLen::Unbounded),
        Rule::unbounded_barcode_segment => GeomPiece::Barcode(GeomLen::Unbounded),
        Rule::unbounded_discard_segment => GeomPiece::Discard(GeomLen::Unbounded),
        Rule::unbounded_read_segment => GeomPiece::ReadSeq(GeomLen::Unbounded),
        _ => unimplemented!(),
    }
}

/// Parses any type of geometry segment.  According to the grammer, this will be either
/// a fixed_segment, fixed_seq_segment, ranged_segment, or unbounded_segment. This function
/// is the top-level parser for individual "pieces" of geometry, and returns the corresponding
/// `GeomPiece`.
pub fn parse_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::fixed_segment => {
            return parse_fixed_segment(r.into_inner().next().unwrap());
        }
        Rule::fixed_seq_segment => {
            let fseq = parse_fixed_seq(&mut r.into_inner());
            return GeomPiece::Fixed(fseq);
        }
        Rule::ranged_segment => {
            return parse_ranged_segment(r.into_inner().next().unwrap());
        }
        Rule::unbounded_segment => {
            return parse_unbounded_segment(r.into_inner().next().unwrap());
        }
        _ => unimplemented!(),
    };
}

/// This trait says that a given implementor is able to properly add itself
/// to the command represented by `cmd`.
pub trait AppendToCmdArgs {
    fn append(&self, cmd: &mut std::process::Command);
}

// ======== for piscem

#[derive(Debug, Eq, PartialEq)]
pub struct PiscemGeomDesc {
    pub read1_desc: String,
    pub read2_desc: String,
}

impl AppendToCmdArgs for PiscemGeomDesc {
    fn append(&self, cmd: &mut std::process::Command) {
        let geo_desc = format!("1{}2{}", self.read1_desc, self.read2_desc);
        cmd.args(["--geometry", geo_desc.as_str()]);
    }
}

fn as_piscem_geom_desc_single_read(geom_pieces: &[GeomPiece]) -> String {
    let desc = geom_pieces
        .iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join("");
    format!("{{{}}}", desc)
}

impl PiscemGeomDesc {
    pub fn from_geom_pieces(geom_pieces_r1: &[GeomPiece], geom_pieces_r2: &[GeomPiece]) -> Self {
        let read1_desc = as_piscem_geom_desc_single_read(geom_pieces_r1);
        let read2_desc = as_piscem_geom_desc_single_read(geom_pieces_r2);
        Self {
            read1_desc,
            read2_desc,
        }
    }
}

// ======== for salmon

#[derive(Debug, Eq, PartialEq)]
pub struct SalmonSeparateGeomDesc {
    pub barcode_desc: String,
    pub umi_desc: String,
    pub read_desc: String,
}

impl AppendToCmdArgs for SalmonSeparateGeomDesc {
    fn append(&self, cmd: &mut std::process::Command) {
        cmd.args([
            "--read-geometry",
            self.read_desc.as_str(),
            "--bc-geometry",
            self.barcode_desc.as_str(),
            "--umi-geometry",
            self.umi_desc.as_str(),
        ]);
    }
}

// for the "separate" salmon format, we need to collect
// the intervals corresponding to each part of the geometry
// separately.  So we need to keep track of intervals which
// is just a pair of offsets.

// the offset can be bounded, or unbounded
// (i.e. goes until the end of the current read)
enum GeomOffset {
    Bounded(u32),
    Unbounded,
}

// an interval is just a pair of offsets
struct GeomInterval {
    start: GeomOffset,
    end: GeomOffset,
}

/// to be able to render a GeomInterval as a string. This
/// is basically just rendering "x-y", for offsets x and y, but
/// if y is unbounded, we render "x-end" instead.
impl fmt::Display for GeomInterval {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        let s = match self.start {
            GeomOffset::Bounded(x) => format!("{}", x),
            _ => "XXX".to_string(),
        };
        let e = match self.end {
            GeomOffset::Bounded(x) => format!("{}", x),
            GeomOffset::Unbounded => "end".to_string(),
        };
        write!(f, "{}-{}", s, e)
    }
}

/// should return struct or enum instead
fn as_salmon_desc_separate_helper(geom_pieces: &[GeomPiece]) -> (String, String, String) {
    let mut offset = 0_u32;

    let mut bc_intervals = Vec::<GeomInterval>::new();
    let mut umi_intervals = Vec::<GeomInterval>::new();
    let mut read_intervals = Vec::<GeomInterval>::new();

    let append_interval_bounded = |offset: &mut u32, x: u32, intervals: &mut Vec<GeomInterval>| {
        let start = *offset + 1;
        let end = *offset + x;
        intervals.push(GeomInterval {
            start: GeomOffset::Bounded(start),
            end: GeomOffset::Bounded(end),
        });
        *offset += x;
    };

    let append_interval_unbounded = |offset: &mut u32, intervals: &mut Vec<GeomInterval>| {
        let start = *offset + 1;
        intervals.push(GeomInterval {
            start: GeomOffset::Bounded(start),
            end: GeomOffset::Unbounded,
        });
    };

    for gp in geom_pieces {
        match gp {
            GeomPiece::Barcode(GeomLen::FixedLen(x)) => {
                append_interval_bounded(&mut offset, *x, &mut bc_intervals);
            }
            GeomPiece::Umi(GeomLen::FixedLen(x)) => {
                append_interval_bounded(&mut offset, *x, &mut umi_intervals);
            }
            GeomPiece::ReadSeq(GeomLen::FixedLen(x)) => {
                append_interval_bounded(&mut offset, *x, &mut read_intervals);
            }
            GeomPiece::Discard(GeomLen::FixedLen(x)) => {
                offset += x;
            }
            GeomPiece::Fixed(NucStr::Seq(_s)) => {
                unimplemented!("Fixed content nucleotide tags are not supported in the salmon separate description format");
            }
            GeomPiece::Barcode(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut bc_intervals);
            }
            GeomPiece::Umi(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut umi_intervals);
            }
            GeomPiece::ReadSeq(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut read_intervals);
            }
            GeomPiece::Discard(GeomLen::Unbounded) => {}
            r => unimplemented!("encountered unexpected GeomPiece {:?}", r),
        };
    }

    let bc_str = bc_intervals
        .iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join(",");

    let umi_str = umi_intervals
        .iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join(",");

    let read_str = read_intervals
        .iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join(",");
    (
        format!("[{}]", bc_str),
        format!("[{}]", umi_str),
        format!("[{}]", read_str),
    )
}

impl SalmonSeparateGeomDesc {
    pub fn from_geom_pieces(geom_pieces_r1: &[GeomPiece], geom_pieces_r2: &[GeomPiece]) -> Self {
        let mut barcode_rep = String::new();
        let mut umi_rep = String::new();
        let mut read_rep = String::new();
        let (bcp, up, rp) = as_salmon_desc_separate_helper(geom_pieces_r1);
        if bcp != "[]" {
            barcode_rep += &format!("1{}", bcp);
        }
        if up != "[]" {
            umi_rep += &format!("1{}", up);
        }
        if rp != "[]" {
            read_rep += &format!("1{}", rp);
        }

        let (bcp, up, rp) = as_salmon_desc_separate_helper(geom_pieces_r2);
        if bcp != "[]" {
            barcode_rep += &format!("2{}", bcp);
        }
        if up != "[]" {
            umi_rep += &format!("2{}", up);
        }
        if rp != "[]" {
            read_rep += &format!("2{}", rp);
        }

        Self {
            barcode_desc: barcode_rep,
            umi_desc: umi_rep,
            read_desc: read_rep,
        }
    }
}

#[derive(Debug)]
pub struct FragmentGeomDesc {
    pub read1_desc: Vec<GeomPiece>,
    pub read2_desc: Vec<GeomPiece>,
}

impl fmt::Display for FragmentGeomDesc {
    /// Write back a geometry fragment specification as exactly
    /// the type of string the parser should accept in the first place.
    /// This is the canonical representation of the geometry.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let desc1 = self
            .read1_desc
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join("");
        let desc2 = self
            .read2_desc
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join("");
        write!(f, "1{{{}}}2{{{}}}", desc1, desc2)
    }
}

impl FragmentGeomDesc {
    /// A "complex" geometry is one that contains
    /// a FixedSeq piece, and/or a BoundedRange piece
    pub fn is_complex_geometry(&self) -> bool {
        for gp in self.read1_desc.iter().chain(self.read2_desc.iter()) {
            if gp.is_complex() {
                return true;
            }
        }
        false
    }

    /// A "complex" geometry is one that contains only fixed length pieces
    /// (but doesn't include any fixed seq segments) and unbounded pieces.
    pub fn is_simple_geometry(&self) -> bool {
        !self.is_complex_geometry()
    }
}

impl<'a> TryFrom<&'a str> for FragmentGeomDesc {
    type Error = anyhow::Error;

    fn try_from(arg: &'a str) -> Result<Self, Self::Error> {
        match FragGeomParser::parse(Rule::frag_desc, arg) {
            Ok(fragment_desc) => {
                //println!("{:#?}", parse);

                let mut read1_desc = Vec::<GeomPiece>::new();
                let mut read2_desc = Vec::<GeomPiece>::new();

                // Because ident_list is silent, the iterator will contain idents
                for read_desc in fragment_desc {
                    // A pair is a combination of the rule which matched and a span of input
                    /*
                    println!("Rule:    {:?}", read_desc.as_rule());
                    println!("Span:    {:?}", read_desc.as_span());
                    println!("Text:    {}", read_desc.as_str());
                    */

                    let read_num = match read_desc.as_rule() {
                        Rule::read_1_desc => 1,
                        Rule::read_2_desc => 2,
                        _ => 0,
                    };
                    // at the top-level we have a
                    // a read 1 desc followed by a read 2 desc
                    for rd in read_desc.into_inner() {
                        match rd.as_rule() {
                            Rule::read_desc => {
                                for geom_piece in rd.into_inner() {
                                    match read_num {
                                        1 => {
                                            read1_desc.push(parse_segment(geom_piece));
                                        }
                                        2 => {
                                            read2_desc.push(parse_segment(geom_piece));
                                        }
                                        _ => {
                                            println!("cannot add geom piece to read {}", read_num);
                                        }
                                    }
                                }
                            }
                            _ => unreachable!(),
                        };
                    }
                }

                Ok(FragmentGeomDesc {
                    read1_desc,
                    read2_desc,
                })
            }
            Err(e) => Err(anyhow!(
                "Could not succesfully parse geometry description {}.\nParse Error : {:#?}",
                arg,
                e
            )),
        }
    }
}
