use crate::{JsonObject, Result};
use serde::{Serialize, Serializer};

/// Serialize the given data structure as a String of JSON.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_feature_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let vec = to_feature_vec(value)?;
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    Ok(string)
}

pub fn to_feature_collection_string<T>(values: &[T]) -> Result<String>
where
    T: Serialize,
{
    let vec = to_feature_collection_vec(values)?;
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    Ok(string)
}

/// Serialize the given data structure as a JSON byte vector.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_feature_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut writer = Vec::with_capacity(128);
    to_feature_writer(&mut writer, value)?;
    Ok(writer)
}

#[inline]
pub fn to_feature_collection_vec<T>(values: &[T]) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut writer = Vec::with_capacity(128);
    to_feature_collection_writer(&mut writer, values)?;
    Ok(writer)
}

use std::io;

/// Serialize the given data structure as JSON into the IO stream.
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail, or if `T` contains a map with non-string keys.
#[inline]
pub fn to_feature_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: Serialize,
{
    let feature_serializer = FeatureWrapper::new(value);
    let mut serializer = serde_json::Serializer::new(writer);
    feature_serializer.serialize(&mut serializer)?;
    Ok(())
}

pub trait SerializableAsFeature<G, P> {
    fn geometry(&self) -> G;
    fn properties(&self) -> P;
}

struct Features<'a, T>
where
    T: Serialize,
{
    features: &'a [T],
}
impl<'a, T> Features<'a, T>
where
    T: Serialize,
{
    fn new(features: &'a [T]) -> Self {
        Self { features }
    }
}

impl<'a, T> serde::Serialize for Features<'a, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        for feature in self.features.iter() {
            seq.serialize_element(&FeatureWrapper::new(feature))?;
        }
        seq.end()
    }
}

struct FeatureWrapper<'t, T> {
    feature: &'t T,
}

impl<'t, T> FeatureWrapper<'t, T> {
    fn new(feature: &'t T) -> Self {
        Self { feature }
    }
}

use serde::ser::Error;

impl<T> Serialize for FeatureWrapper<'_, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut json_object: JsonObject = {
            // PERF: this feels like an extra round-trip just to juggle some fields around.
            // How can we skip this?
            //let bytes = serde_json::to_vec(self.feature).map_err(|e| S::Error::from(Box::new(e)))?;
            let bytes = serde_json::to_vec(self.feature)
                .map_err(|e| S::Error::custom(format!("unable to serialize to json: {}", e)))?;
            serde_json::from_slice(&bytes)
                .map_err(|e| S::Error::custom(format!("unable to roundtrip from json: {}", e)))?
        };
        let geometry = json_object.remove("geometry");

        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("type", "Feature")?;
        map.serialize_entry("geometry", &geometry)?;
        map.serialize_entry("properties", &json_object)?;
        map.end()
    }
}

#[inline]
pub fn to_feature_collection_writer<W, T>(writer: W, features: &[T]) -> Result<()>
where
    W: io::Write,
    T: Serialize,
{
    use serde::ser::SerializeMap;

    let mut ser = serde_json::Serializer::new(writer);
    let mut map = ser.serialize_map(Some(2))?;
    map.serialize_entry("type", "FeatureCollection")?;
    map.serialize_entry("features", &Features::new(features))?;
    map.end()?;
    Ok(())
}

fn serialize_geometry<IG, S>(geometry: IG, ser: S) -> std::result::Result<S::Ok, S::Error>
where
    IG: std::convert::TryInto<crate::Geometry>,
    S: serde::Serializer,
{
    geometry
        .try_into()
        .map_err(|_e| Error::custom(format!("failed to convert geometry to geojson")))
        .and_then(|geojson_geometry| geojson_geometry.serialize(ser))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JsonObject, JsonValue};
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn happy_path() {
        #[derive(Serialize)]
        struct MyStruct {
            geometry: crate::Geometry,
            name: String,
        }

        let my_feature = {
            let geometry = crate::Geometry::new(crate::Value::Point(vec![0.0, 1.0]));
            let name = "burbs".to_string();
            MyStruct { geometry, name }
        };

        let expected_output_json = json!({
            "type": "Feature",
            "geometry": {
                "coordinates":[0.0,1.0],
                "type":"Point"
            },
            "properties": {
                "name": "burbs"
            }
        });

        let actual_output = to_feature_string(&my_feature).unwrap();
        let actual_output_json = JsonValue::from_str(&actual_output).unwrap();
        assert_eq!(actual_output_json, expected_output_json);
    }

    mod optional_geometry {
        use super::*;
        #[derive(Serialize)]
        struct MyStruct {
            geometry: Option<crate::Geometry>,
            name: String,
        }

        #[test]
        fn with_some_geom() {
            let my_feature = {
                let geometry = Some(crate::Geometry::new(crate::Value::Point(vec![0.0, 1.0])));
                let name = "burbs".to_string();
                MyStruct { geometry, name }
            };

            let expected_output_json = json!({
                "type": "Feature",
                "geometry": {
                    "coordinates":[0.0,1.0],
                    "type":"Point"
                },
                "properties": {
                    "name": "burbs"
                }
            });

            let actual_output = to_feature_string(&my_feature).unwrap();
            let actual_output_json = JsonValue::from_str(&actual_output).unwrap();
            assert_eq!(actual_output_json, expected_output_json);
        }

        #[test]
        fn with_no_geom() {
            let my_feature = {
                let geometry = None;
                let name = "burbs".to_string();
                MyStruct { geometry, name }
            };

            let expected_output_json = json!({
                "type": "Feature",
                "geometry": null,
                "properties": {
                    "name": "burbs"
                }
            });

            let actual_output = to_feature_string(&my_feature).unwrap();
            let actual_output_json = JsonValue::from_str(&actual_output).unwrap();
            assert_eq!(actual_output_json, expected_output_json);
        }
    }

    #[cfg(feature = "geo-types")]
    mod geo_types_tests {
        use super::*;

        fn feature_collection_string() -> String {
            json!({
                "type": "FeatureCollection",
                "features": [
                    {
                      "type": "Feature",
                      "geometry": {
                        "type": "Point",
                        "coordinates": [125.6, 10.1]
                      },
                      "properties": {
                        "name": "Dinagat Islands",
                        "age": 123
                      }
                    },
                    {
                      "type": "Feature",
                      "geometry": {
                        "type": "Point",
                        "coordinates": [2.3, 4.5]
                      },
                      "properties": {
                        "name": "Neverland",
                        "age": 456
                      }
                    }
                ]
            })
            .to_string()
        }

        #[test]
        fn geometry_field() {
            // Some example object, that we want to parse the geojson into.
            #[derive(Serialize)]
            struct MyStruct {
                #[serde(serialize_with = "serialize_geometry")]
                geometry: geo_types::Point<f64>,
                name: String,
                age: u64,
            }

            let my_struct = MyStruct {
                geometry: geo_types::point!(x: 125.6, y: 10.1).into(),
                name: "Dinagat Islands".to_string(),
                age: 123,
            };

            let expected_output = serde_json::json!({
              "type": "Feature",
              "geometry": {
                "type": "Point",
                "coordinates": [125.6, 10.1]
              },
              "properties": {
                "name": "Dinagat Islands",
                "age": 123
              }
            });

            // Order might vary, so re-parse to do a semantic comparison of the content.
            let output_string = to_feature_string(&my_struct).expect("valid serialization");
            let actual_output = JsonValue::from_str(&output_string).unwrap();

            assert_eq!(actual_output, expected_output);
        }

        #[test]
        fn feature_collection() {
            // Some example object, that we want to parse the geojson into.
            #[derive(Serialize)]
            struct MyStruct {
                #[serde(serialize_with = "serialize_geometry")]
                geometry: geo_types::Point<f64>,
                name: String,
                age: u64,
            }

            impl SerializableAsFeature<crate::Geometry, crate::JsonObject> for MyStruct {
                fn geometry(&self) -> crate::Geometry {
                    (&self.geometry).into()
                }

                fn properties(&self) -> crate::JsonObject {
                    let mut map = JsonObject::new();
                    map.insert("name".to_string(), self.name.clone().into());
                    map.insert("age".to_string(), self.age.into());
                    map
                }
            }

            let my_structs = vec![
                MyStruct {
                    geometry: geo_types::point!(x: 125.6, y: 10.1),
                    name: "Dinagat Islands".to_string(),
                    age: 123,
                },
                MyStruct {
                    geometry: geo_types::point!(x: 2.3, y: 4.5),
                    name: "Neverland".to_string(),
                    age: 456,
                },
            ];

            let output_string =
                to_feature_collection_string(&my_structs).expect("valid serialization");

            // Order might vary, so re-parse to do a semantic comparison of the content.
            let expected_output = JsonValue::from_str(&feature_collection_string()).unwrap();
            let actual_output = JsonValue::from_str(&output_string).unwrap();

            assert_eq!(actual_output, expected_output);
        }
    }
}
