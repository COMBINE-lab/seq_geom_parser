extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;
use std::fmt;

#[derive(Parser)]
#[grammar = "grammar/frag_geom.pest"] // relative to src
struct FragGeomParser;

#[derive(Debug, Copy, Clone)]
enum GeomLen {
    Bounded(u32),
    Unbounded,
}

#[derive(Debug, Copy, Clone)]
enum GeomPiece {
    Barcode(GeomLen),
    UMI(GeomLen),
    Discard(GeomLen),
    ReadSeq(GeomLen),
}

fn parse_bounded_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::bounded_umi_segment => {
            for len_val in r.into_inner() {
                return GeomPiece::UMI(GeomLen::Bounded(len_val.as_str().parse::<u32>().unwrap()));
            }
        }
        Rule::bounded_barcode_segment => {
            for len_val in r.into_inner() {
                return GeomPiece::Barcode(GeomLen::Bounded(
                    len_val.as_str().parse::<u32>().unwrap(),
                ));
            }
        }
        Rule::bounded_discard_segment => {
            for len_val in r.into_inner() {
                return GeomPiece::Discard(GeomLen::Bounded(
                    len_val.as_str().parse::<u32>().unwrap(),
                ));
            }
        }
        Rule::bounded_read_segment => {
            for len_val in r.into_inner() {
                return GeomPiece::ReadSeq(GeomLen::Bounded(
                    len_val.as_str().parse::<u32>().unwrap(),
                ));
            }
        }
        _ => unimplemented!(),
    };
    return GeomPiece::Discard(GeomLen::Unbounded);
}

fn parse_unbounded_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::unbounded_umi_segment => {
            return GeomPiece::UMI(GeomLen::Unbounded);
        }
        Rule::unbounded_barcode_segment => {
            return GeomPiece::Barcode(GeomLen::Unbounded);
        }
        Rule::unbounded_discard_segment => {
            return GeomPiece::Discard(GeomLen::Unbounded);
        }
        Rule::unbounded_read_segment => {
            return GeomPiece::ReadSeq(GeomLen::Unbounded);
        }
        _ => unimplemented!(),
    };
}

fn parse_segment(r: pest::iterators::Pair<Rule>) -> GeomPiece {
    match r.as_rule() {
        Rule::bounded_segment => {
            return parse_bounded_segment(r.into_inner().next().unwrap());
        }
        Rule::unbounded_segment => {
            return parse_unbounded_segment(r.into_inner().next().unwrap());
        }
        _ => unimplemented!(),
    };
}

fn as_piscem_str(geom_pieces: &[GeomPiece]) -> String {
    let mut rep = String::from("{");
    for gp in geom_pieces {
        match gp {
            GeomPiece::Discard(GeomLen::Bounded(x)) => {
                rep += &format!("x[{}]", x);
            }
            GeomPiece::Barcode(GeomLen::Bounded(x)) => {
                rep += &format!("b[{}]", x);
            }
            GeomPiece::UMI(GeomLen::Bounded(x)) => {
                rep += &format!("u[{}]", x);
            }
            GeomPiece::ReadSeq(GeomLen::Bounded(x)) => {
                rep += &format!("r[{}]", x);
            }
            GeomPiece::Discard(GeomLen::Unbounded) => {
                rep += "x:";
            }
            GeomPiece::Barcode(GeomLen::Unbounded) => {
                rep += "b:";
            }
            GeomPiece::UMI(GeomLen::Unbounded) => {
                rep += "u:";
            }
            GeomPiece::ReadSeq(GeomLen::Unbounded) => {
                rep += "r:";
            }
        }
    }
    rep += "}";
    rep
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
            _ => format!("XXX"),
        };
        let e = match self.end {
            GeomOffset::Bounded(x) => format!("{}", x),
            GeomOffset::Unbounded => format!("end"),
        };
        write!(f, "{}-{}", s, e)
    }
}

/// should return struct or enum instead
fn as_salmon_str_separate_helper(geom_pieces: &[GeomPiece]) -> (String, String, String) {
    let mut offset = 1_u32;

    let mut bc_intervals = Vec::<GeomInterval>::new();
    let mut umi_intervals = Vec::<GeomInterval>::new();
    let mut read_intervals = Vec::<GeomInterval>::new();

    let append_interval_bounded =
        |offset: &mut u32, x: u32, intervals: &mut Vec<GeomInterval>| -> () {
            let start = offset.clone();
            let end = *offset + x;
            intervals.push(GeomInterval {
                start: GeomOffset::Bounded(start),
                end: GeomOffset::Bounded(end),
            });
            *offset += x;
        };

    let append_interval_unbounded = |offset: &mut u32, intervals: &mut Vec<GeomInterval>| -> () {
        let start = offset;
        intervals.push(GeomInterval {
            start: GeomOffset::Bounded(*start),
            end: GeomOffset::Unbounded,
        });
    };

    for gp in geom_pieces {
        match gp {
            GeomPiece::Barcode(GeomLen::Bounded(x)) => {
                append_interval_bounded(&mut offset, *x, &mut bc_intervals);
            }
            GeomPiece::UMI(GeomLen::Bounded(x)) => {
                append_interval_bounded(&mut offset, *x, &mut umi_intervals);
            }
            GeomPiece::ReadSeq(GeomLen::Bounded(x)) => {
                append_interval_bounded(&mut offset, *x, &mut read_intervals);
            }
            GeomPiece::Discard(GeomLen::Bounded(x)) => {
                offset += x;
            }
            GeomPiece::Barcode(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut bc_intervals);
            }
            GeomPiece::UMI(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut umi_intervals);
            }
            GeomPiece::ReadSeq(GeomLen::Unbounded) => {
                append_interval_unbounded(&mut offset, &mut read_intervals);
            }
            GeomPiece::Discard(GeomLen::Unbounded) => {}
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

fn as_salmon_str_separate(geom_pieces_r1: &[GeomPiece], geom_pieces_r2: &[GeomPiece]) -> String {
    let mut barcode_rep = String::from("--barcode_geometry ");
    let mut umi_rep = String::from("--umi_geometry ");
    let mut read_rep = String::from("--read_geometry ");
    let (bcp, up, rp) = as_salmon_str_separate_helper(&geom_pieces_r1);
    if bcp != "[]" {
        barcode_rep += &format!("1{}", bcp);
    }
    if up != "[]" {
        umi_rep += &format!("1{}", up);
    }
    if rp != "[]" {
        read_rep += &format!("1{}", rp);
    }

    let (bcp, up, rp) = as_salmon_str_separate_helper(&geom_pieces_r2);
    if bcp != "[]" {
        barcode_rep += &format!("2{}", bcp);
    }
    if up != "[]" {
        umi_rep += &format!("2{}", up);
    }
    if rp != "[]" {
        read_rep += &format!("2{}", rp);
    }

    format!("{} {} {}", barcode_rep, umi_rep, read_rep)
}

fn main() {
    let arg = std::env::args().nth(1).unwrap();
    println!("arg = {}", &arg);
    let fragment_desc = FragGeomParser::parse(Rule::frag_desc, &arg).expect("unsuccessful parse");
    //println!("{:#?}", parse);

    let mut read1_desc = Vec::<GeomPiece>::new();
    let mut read2_desc = Vec::<GeomPiece>::new();

    // Because ident_list is silent, the iterator will contain idents
    for read_desc in fragment_desc {
        // A pair is a combination of the rule which matched and a span of input
        println!("Rule:    {:?}", read_desc.as_rule());
        println!("Span:    {:?}", read_desc.as_span());
        println!("Text:    {}", read_desc.as_str());

        let read_num : u32;
        match read_desc.as_rule() {
            Rule::read_1_desc => {
                read_num = 1;
            }
            Rule::read_2_desc => {
                read_num = 2;
            }
            _ => unreachable!(),
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

    //println!("r1 : {:#?}", read1_desc);
    //println!("r2 : {:#?}", read2_desc);

    println!("piscem(r1) : 1{}", as_piscem_str(&read1_desc));
    println!("piscem(r2) : 2{}", as_piscem_str(&read2_desc));

    println!(
        "salmon(r1,r2) : {}",
        as_salmon_str_separate(&read1_desc, &read2_desc)
    );
}
