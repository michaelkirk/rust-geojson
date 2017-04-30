extern crate geojson;
extern crate serde_json;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
		use std::fs::File;
		use std::io::prelude::*;
		use std::fs;
		use geojson::GeoJson;
        use serde_json::Value;
        use serde_json;

		let fixture_dir_path = "tests/fixtures/good";

		for dir_entry in fs::read_dir(fixture_dir_path).unwrap() {
			let path = dir_entry.unwrap().path();
			let path_str = path.to_str().unwrap();

			//println!("Name: {}", &path_str);

			let mut file = File::open(&path).unwrap();
			let mut file_contents = String::new();
			file.read_to_string(&mut file_contents);

			//println!("Contents: {}", &file_contents);

            let geojson_result = file_contents.parse::<GeoJson>();

            match geojson_result {
                Ok(geojson) => {
                    println!(">> parse success! file: {}", &path_str);

                    // Now that we've successfully decoded the geojson, re-encode it
                    // to and compare to the original to make sure nothing was lost.
                    let geojson_string = geojson.to_string();

                    let original_json: serde_json::Value = serde_json::from_str(&file_contents).unwrap();
                    let roundtrip_json: serde_json::Value = serde_json::from_str(&geojson_string).unwrap();

                    if original_json == roundtrip_json {
                        println!(">> roundtrip success! file: {}", &path_str);
                    } else {
                        println!("<<< roundtrip failure! file: {}", &path_str);
                        println!("<<< roundtrip failure! expected: {}", &original_json);
                        println!("<<< roundtrip failure! found: {}", &roundtrip_json);
                    }
 
                }
                Err(error) => {
                    println!("<<< parse failure! file: {} error: {}", &path_str, &error);
                    // FIXME actually fail here.
                    // TODO wrap in custom error, attaching failing file
                    //assert!(false)
                }
            };

            //let geojson_string = geojson_result.unwrap().to_string();

            //let original_json: serde_json::Value = serde_json::from_str(&file_contents).unwrap();
            //let roundtrip_json: serde_json::Value = serde_json::from_str(&geojson_string).unwrap();

            //if (original_json == roundtrip_json) {
                //println!(">> roundtrip success! file: {}", &path_str);
            //} else {
                //println!(">> roundtrip failure! file: {}", &path_str);
            //}
            ////assert_eq!(original_json, roundtrip_json);
		}
    }
}
