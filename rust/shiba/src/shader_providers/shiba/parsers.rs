use super::types::*;
use crate::parsers::{glsl::*, *};
use nom::{
	branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, sequence::*,
	IResult,
};

fn section(input: &str) -> IResult<&str, Section> {
	directive(alt((
		value(Section::Attributes, tag("attributes")),
		value(Section::Common, tag("common")),
		map(fragment_directive, Section::Fragment),
		value(Section::Outputs, tag("outputs")),
		value(Section::Varyings, tag("varyings")),
		map(vertex_directive, Section::Vertex),
	)))(input)
}

fn sections(input: &str) -> IResult<&str, Vec<(&str, Section)>> {
	many0(take_unless(map(section, Some)))(input)
}

fn version(input: &str) -> IResult<&str, &str> {
	let (input, _) = once()(input)?;
	let (input, _) = tag("#version")(input)?;
	let (input, _) = space1(input)?;
	let (input, version) = not_line_ending(input)?;
	let (input, _) = line_ending(input)?;
	Ok((input, version))
}

pub type Contents<'a> = (Option<&'a str>, Vec<(&'a str, Section)>);

pub fn contents(input: &str) -> IResult<&str, Contents> {
	tuple((opt(version), sections))(input)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_contents() {
		let contents = contents(
			r#"#version 450
#define foo bar
prolog code
#pragma shiba common
common code
#pragma shiba vertex 42
vertex code
"#,
		);

		assert_eq!(
			contents,
			Ok((
				"vertex code\n",
				(
					Some("450"),
					vec![
						("#define foo bar\nprolog code\n", Section::Common),
						("common code\n", Section::Vertex(42)),
					]
				)
			))
		);
	}
}
