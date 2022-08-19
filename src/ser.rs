#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JsonObject, JsonValue};
    use serde_json::json;
    use std::str::FromStr;

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

            let expected_string = serde_json::json!({
              "type": "Feature",
              "geometry": {
                "type": "Point",
                "coordinates": [125.6, 10.1]
              },
              "properties": {
                "name": "Dinagat Islands",
                "age": 123
              }
            })
            .to_string();
            let output_string = to_feature_string(&my_struct).expect("valid serialization");

            assert_eq!(output_string, expected_string);
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
                    geometry: geo_types::point!(x: 125.6, y: 10.1).into(),
                    name: "Dinagat Islands".to_string(),
                    age: 123,
                },
                MyStruct {
                    geometry: geo_types::point!(x: 2.3, y: 4.5).into(),
                    name: "Neverland".to_string(),
                    age: 456,
                },
            ];

            let output_string =
                to_feature_collection_string(&my_structs).expect("valid serialization");

            // Order might vary, so re-parse to do a semantic comparison of the content.
            let expected = JsonValue::from_str(&feature_collection_string()).unwrap();
            let output = JsonValue::from_str(&output_string).unwrap();

            assert_eq!(output, expected);
        }
    }
}

use crate::Result;
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

pub fn to_feature_collection_string<T, G, P>(values: &[T]) -> Result<String>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
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
pub fn to_feature_collection_vec<T, G, P>(values: &[T]) -> Result<Vec<u8>>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    let mut writer = Vec::with_capacity(128);
    to_feature_collection_writer(&mut writer, values)?;
    Ok(writer)
}

use std::io;
use std::marker::PhantomData;

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
    let mut tmp = vec![];
    let mut ser = serde_json::Serializer::new(&mut tmp);

    value.serialize(&mut ser).unwrap();
    let json_string = String::from_utf8(tmp).expect("valid utf-8");

    use std::str::FromStr;
    let mut properties =
        crate::util::expect_owned_object(crate::JsonValue::from_str(&json_string)?)?;

    use std::convert::TryFrom;
    let geometry_object = properties.remove("geometry").unwrap();
    let geometry = crate::Geometry::try_from(geometry_object).unwrap();

    let feature = crate::Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: Some(properties),
        foreign_members: None,
    };

    let mut re_ser = serde_json::Serializer::new(writer);
    feature.serialize(&mut re_ser).unwrap();

    Ok(())
}

pub trait SerializableAsFeature<G, P>
where
    G: Serialize,
    P: Serialize,
{
    fn geometry(&self) -> G;
    fn properties(&self) -> P;
}

struct Features<'a, T, G, P>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    features: &'a [T],
    geometry: PhantomData<G>,
    properties: PhantomData<P>,
}
impl<'a, T, G, P> Features<'a, T, G, P>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    fn new(features: &'a [T]) -> Self {
        Self {
            features,
            geometry: PhantomData,
            properties: PhantomData,
        }
    }
}

struct FeatureSerializer;

impl<'a, T, G, P> serde::Serialize for Features<'a, T, G, P>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
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

struct FeatureWrapper<'t, T, G, P> {
    feature: &'t T,
    geometry: PhantomData<G>,
    properties: PhantomData<P>,
}

impl<'t, T, G, P> FeatureWrapper<'t, T, G, P> {
    fn new(feature: &'t T) -> Self {
        Self {
            feature,
            geometry: PhantomData,
            properties: PhantomData,
        }
    }
}

impl<T, G, P> FeatureWrapper<'_, T, G, P>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    fn geometry(&self) -> G {
        self.feature.geometry()
    }

    fn properties(&self) -> P {
        self.feature.properties()
    }
}

impl<T, G, P> Serialize for FeatureWrapper<'_, T, G, P>
where
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", "Feature")?;
        map.serialize_entry("geometry", &self.geometry())?;
        map.serialize_entry("properties", &self.properties())?;
        map.end()
    }
}

#[inline]
pub fn to_feature_collection_writer<W, T, G, P>(writer: W, values: &[T]) -> Result<()>
where
    W: io::Write,
    T: SerializableAsFeature<G, P>,
    G: Serialize,
    P: Serialize,
{
    use serde::ser::SerializeMap;

    let mut ser = serde_json::Serializer::new(writer);
    let mut map = ser.serialize_map(Some(2))?;
    map.serialize_entry("type", "FeatureCollection")?;
    let features = Features::new(values);
    map.serialize_entry("features", &features)?;
    map.end()?;
    Ok(())
}

fn serialize_geometry<IG, S>(geometry: IG, ser: S) -> std::result::Result<S::Ok, S::Error>
where
    IG: std::convert::TryInto<crate::Geometry>,
    S: serde::Serializer,
{
    use serde::ser::Error;
    geometry
        .try_into()
        .map_err(|_e| Error::custom(format!("failed to convert geometry to geojson")))
        .and_then(|geojson_geometry| geojson_geometry.serialize(ser))
}
