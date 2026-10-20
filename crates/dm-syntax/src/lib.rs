//! Lossless syntax tree for Dialogue Manager `.dialogue` files.
//!
//! The first format adapter. Every file round-trips byte for byte:
//! `print(parse(file)) == file`. Syntax the parser does not understand
//! becomes an opaque node kept verbatim. The parser arrives with M1.
