// Copyright 2015 The GeoRust Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;
use std::str::FromStr;

use json::{Serialize, Deserialize, Serializer, Deserializer, JsonObject};

use {Error, Geometry, Feature, FeatureCollection, FromObject};


/// GeoJSON Objects
///
/// [GeoJSON Format Specification § 2]
/// (http://geojson.org/geojson-spec.html#geojson-objects)
#[derive(Clone, Debug, PartialEq)]
pub enum GeoJson {
    Geometry(Geometry),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
}

impl<'a> From<&'a GeoJson> for JsonObject {
    fn from(geojson: &'a GeoJson) -> JsonObject {
        return match *geojson {
            GeoJson::Geometry(ref geometry) => geometry.into(),
            GeoJson::Feature(ref feature) => feature.into(),
            GeoJson::FeatureCollection(ref fc) => fc.into(),
        };
    }
}

impl From<Geometry> for GeoJson {
    fn from(geometry: Geometry) -> Self {
        GeoJson::Geometry(geometry)
    }
}

impl From<Feature> for GeoJson {
    fn from(feature: Feature) -> Self {
        GeoJson::Feature(feature)
    }
}

impl From<FeatureCollection> for GeoJson {
    fn from(feature_collection: FeatureCollection) -> GeoJson {
        GeoJson::FeatureCollection(feature_collection)
    }
}


impl FromObject for GeoJson {
    fn from_object(object: &JsonObject) -> Result<Self, Error> {
        let type_ = expect_string!(expect_property!(object, "type", "Missing 'type' field"));
        return match &type_ as &str {
            "Point" |
            "MultiPoint" |
            "LineString" |
            "MultiLineString" |
            "Polygon" |
            "MultiPolygon" => Geometry::from_object(object).map(GeoJson::Geometry),
            "Feature" => Feature::from_object(object).map(GeoJson::Feature),
            "FeatureCollection" => {
                FeatureCollection::from_object(object).map(GeoJson::FeatureCollection)
            }
            _ => Err(Error::GeoJsonUnknownType),
        };
    }
}

impl Serialize for GeoJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        JsonObject::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GeoJson {
    fn deserialize<D>(deserializer: D) -> Result<GeoJson, D::Error>
        where D: Deserializer<'de>
    {
        use std::error::Error as StdError;
        use serde::de::Error as SerdeError;

        let val = try!(JsonObject::deserialize(deserializer));

        GeoJson::from_object(&val).map_err(|e| D::Error::custom(e.description()))
    }
}

impl FromStr for GeoJson {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let object = try!(get_object(s));

        return GeoJson::from_object(&object);
    }
}

fn get_object(s: &str) -> Result<JsonObject, Error> {
    let decoded_json: ::serde_json::Value = match ::serde_json::from_str(s) {
        Ok(j) => j,
        Err(..) => return Err(Error::MalformedJson),
    };

    if let Some(geo) = decoded_json.as_object() {
        return Ok(geo.clone());
    } else {
        return Err(Error::MalformedJson);
    }
}

impl fmt::Display for GeoJson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ::serde_json::to_string(self)
            .map_err(|_| fmt::Error)
            .and_then(|s| f.write_str(&s))
    }
}
