extern crate geojson;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
		use std::fs::File;
		use std::io::prelude::*;
		use std::fs;
		use geojson::GeoJson;

		let fixture_dir_path = "tests/fixtures/good";

		for dir_entry in fs::read_dir(fixture_dir_path).unwrap() {
			// to see printed output: `cargo test -- --nocapture`
			let path = dir_entry.unwrap().path();
			let path_str = path.to_str().unwrap();
			println!("Name: {}", &path_str);
			let mut file = File::open(&path).unwrap();
			let mut file_contents = String::new();
			file.read_to_string(&mut file_contents);

			//println!("Contents: {}", &file_contents);
            match file_contents.parse::<GeoJson>() {
                Ok(geojson) => println!(">> success! file: {}", &path_str),
                Err(error) => {
                    println!(">> failure! file: {} error: {}", &path_str, &error); 
                    assert!(false)
                }
            };
		}
    }
}
