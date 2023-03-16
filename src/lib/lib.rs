//! This is a crate for parsing and interpreting sequence fragment
//! geometry specifications in the sequence
//! [fragment geometry description language](https://hackmd.io/@PI7Og0l1ReeBZu_pjQGUQQ/rJMgmvr13) (FGDL).
//! The FGDL describes how sequenced fragments are laid out, and how different parts of the sequence correspond
//! to technical tags or to biological sequence.  It provides a standard and unified way to represent
//! the sequence layouts used in many different sequencing protocols, and is currently developed with
//! a focus on representing different single-cell sequencing chemistries.
//!
//! This crate provides a library for parsing these descriptions, and a set of structures for representing
//! them in memory, as well as some common traits for transforming and printing them.

extern crate pest;
#[macro_use]
extern crate pest_derive;

use anyhow::{anyhow, bail, Result};
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
    /// This piece of geometry has a single fixed length
    FixedLen(u32),
    /// This piece of geometry has some length between
    /// a provided lower and upper bound
    LenRange(u32, u32),
    /// This piece of geometry has a length whose bound is
    /// not known at geometry specification time
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
    /// A cellular barcode
    Barcode(GeomLen),
    /// A unique molecular identifier
    Umi(GeomLen),
    /// Sequence that will be discarded
    Discard(GeomLen),
    /// Biological read sequence
    ReadSeq(GeomLen),
    /// A fixed sequence anchor / motif
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
            GeomLen::LenRange(l, h)
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
            NucStr::Seq(seq_str.to_owned())
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
            GeomPiece::Umi(gl)
        }
        Rule::ranged_barcode_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            GeomPiece::Barcode(gl)
        }
        Rule::ranged_discard_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            GeomPiece::Discard(gl)
        }
        Rule::ranged_read_segment => {
            let gl = parse_ranged_len(&mut r.into_inner());
            GeomPiece::ReadSeq(gl)
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
            GeomPiece::Umi(gl)
        }
        Rule::fixed_barcode_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            GeomPiece::Barcode(gl)
        }
        Rule::fixed_discard_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            GeomPiece::Discard(gl)
        }
        Rule::fixed_read_segment => {
            let gl = parse_fixed_len(&mut r.into_inner());
            GeomPiece::ReadSeq(gl)
        }
        Rule::fixed_seq_segment => {
            let fseq = parse_fixed_seq(&mut r.into_inner());
            GeomPiece::Fixed(fseq)
        }
        _ => unimplemented!(),
    }
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
        Rule::fixed_segment => parse_fixed_segment(r.into_inner().next().unwrap()),
        Rule::fixed_seq_segment => {
            let fseq = parse_fixed_seq(&mut r.into_inner());
            GeomPiece::Fixed(fseq)
        }
        Rule::ranged_segment => parse_ranged_segment(r.into_inner().next().unwrap()),
        Rule::unbounded_segment => parse_unbounded_segment(r.into_inner().next().unwrap()),
        _ => unimplemented!(),
    }
}

/// This trait says that a given implementor is able to properly add itself
/// to the command represented by `cmd`.
pub trait AppendToCmdArgs {
    fn append(&self, cmd: &mut std::process::Command);
}

// ======== for piscem

/// This struct holds a [`piscem`](https://github.com/COMBINE-lab/piscem) compatible
/// description of the fragment geometry specification.
#[derive(Debug, Eq, PartialEq)]
pub struct PiscemGeomDesc {
    /// The `piscem` format specification for read 1.
    pub read1_desc: String,
    /// The `piscem` format specification for read 2.
    pub read2_desc: String,
}

impl AppendToCmdArgs for PiscemGeomDesc {
    /// Adds this `piscem` format geometry specification to the command
    /// given by `cmd`.
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
    /// This constructor builds the `piscem` format descriptor for this fragment
    /// library from a slice of the constituent `GeomPiece`s for read 1 (`geom_pieces_r1`)
    /// and a slice of the `GeomPiece`s for read 2 (`geom_pieces_r2`).
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

/// This struct holds a [`salmon`](https://github.com/COMBINE-lab/salmon) compatible
/// description of the fragment geometry specification.
#[derive(Debug, Eq, PartialEq)]
pub struct SalmonSeparateGeomDesc {
    pub barcode_desc: String,
    pub umi_desc: String,
    pub read_desc: String,
}

impl AppendToCmdArgs for SalmonSeparateGeomDesc {
    /// Given the `salmon` compatible geometry description, append this description
    /// to the command `cmd`, assumed to be an invocation of `salmon alevin`.
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

/// This structure holds our representation of the parsed fragment
/// geometry description.
#[derive(Debug)]
pub struct FragmentGeomDesc {
    /// The sequence of `GeomPiece`s describing read 1 of this fragment in left-to-right order.
    pub read1_desc: Vec<GeomPiece>,
    /// The sequence of `GeomPiece`s describing read 2 of this fragment in left-to-right order.
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

    /// A "simple" geometry is one that contains only fixed length pieces
    /// (but doesn't include any fixed seq segments) and unbounded pieces.
    pub fn is_simple_geometry(&self) -> bool {
        !self.is_complex_geometry()
    }
}

/// Parse the description of a single read.  It's expected that this function is called
/// when the enclosing rule matches a rule description.  In that case, this function is
/// called with the `into_inner` of that Pair.  This function returns a vector containing
/// the parsed geometry of the input description.
fn parse_read_description(read_desc: pest::iterators::Pairs<Rule>) -> Vec<GeomPiece> {
    let mut read_geom = Vec::<GeomPiece>::new();
    for rd in read_desc {
        match rd.as_rule() {
            Rule::read_desc => {
                for geom_piece in rd.into_inner() {
                    read_geom.push(parse_segment(geom_piece));
                }
            }
            _ => unreachable!(),
        };
    }
    read_geom
}

impl<'a> TryFrom<&'a str> for FragmentGeomDesc {
    type Error = anyhow::Error;

    /// This is the main entry point to obtain a `FragmentGeomDesc` structure.
    /// This function parses the FGDL description string provided as `arg`, and
    /// returns either `Ok(FragGeomDesc)`, if the parse is succesful or an
    /// `anyhow::Error` if the parsing fails.
    ///
    /// Currently, the FGDL makes a structural assumption that is reflected in the
    /// way this function works.  The description string will describe the fragment
    /// geometry for a fragment consisting of a pair of reads (i.e. currently
    /// there is no support for single-end reads or fragments containing > 2 reads).
    fn try_from(arg: &'a str) -> Result<Self, Self::Error> {
        match FragGeomParser::parse(Rule::frag_desc, arg) {
            Ok(fragment_desc) => {
                // Where we'll hold the `GeomPiece`s that constitute the
                // parse of each read.
                let mut r1_desc = None;
                let mut r2_desc = None;

                // Because ident_list is silent, the iterator will contain idents
                for read_desc in fragment_desc {
                    match read_desc.as_rule() {
                        Rule::read_1_desc => {
                            let rd = read_desc.into_inner();
                            r1_desc = Some(parse_read_description(rd));
                        }
                        Rule::read_2_desc => {
                            let rd = read_desc.into_inner();
                            r2_desc = Some(parse_read_description(rd));
                        }
                        Rule::EOI => {}
                        e => {
                            dbg!("{:?}", e);
                            bail!("Expected to parse a description for read 1, or 2, but didn't find the corresponding rule!")
                        }
                    };
                }

                if let (Some(read1_desc), Some(read2_desc)) = (r1_desc, r2_desc) {
                    Ok(FragmentGeomDesc {
                        read1_desc,
                        read2_desc,
                    })
                } else {
                    bail!("Was not able to obtain a succesful parse for both read 1 and read 2.")
                }
            }
            Err(e) => Err(anyhow!(
                "Could not succesfully parse geometry description {}.\nParse Error : {:#?}",
                arg,
                e
            )),
        }
    }
}
