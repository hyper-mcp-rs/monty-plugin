use monty::{MontyObject, ResourceLimits};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginMontyObject(pub MontyObject);

impl From<MontyObject> for PluginMontyObject {
    fn from(value: MontyObject) -> Self {
        Self(value)
    }
}

impl From<PluginMontyObject> for MontyObject {
    fn from(value: PluginMontyObject) -> Self {
        value.0
    }
}

impl AsRef<MontyObject> for PluginMontyObject {
    fn as_ref(&self) -> &MontyObject {
        &self.0
    }
}

impl std::ops::Deref for PluginMontyObject {
    type Target = MontyObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PluginMontyObject {
    pub fn into_inner(self) -> MontyObject {
        self.0
    }
}

impl JsonSchema for PluginMontyObject {
    fn inline_schema() -> bool {
        false
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        fn defs_ref(generator: &SchemaGenerator, name: &str) -> String {
            let path = generator.settings().definitions_path.as_ref();
            let trimmed = path.trim_start_matches('#').trim_matches('/');
            format!("#/{trimmed}/{name}")
        }

        generator.definitions_mut().entry("MontyExcType".to_string())
                .or_insert_with(|| {json!({
                    "description": "Python exception types supported by the interpreter.",
                    "oneOf": [
                        {
                            "type": "string",
                            "enum": [
                                "BaseException",
                                "SystemExit",
                                "KeyboardInterrupt",
                                "ArithmeticError",
                                "OverflowError",
                                "ZeroDivisionError",
                                "LookupError",
                                "IndexError",
                                "KeyError",
                                "RuntimeError",
                                "NotImplementedError",
                                "RecursionError",
                                "AttributeError",
                                "FrozenInstanceError",
                                "NameError",
                                "UnboundLocalError",
                                "ValueError",
                                "UnicodeDecodeError",
                                "ImportError",
                                "ModuleNotFoundError",
                                "OSError",
                                "FileNotFoundError",
                                "FileExistsError",
                                "IsADirectoryError",
                                "NotADirectoryError",
                                "AssertionError",
                                "MemoryError",
                                "StopIteration",
                                "SyntaxError",
                                "TimeoutError",
                                "TypeError"
                            ]
                        },
                        {
                            "description": "Primary exception class — matches any exception in isinstance checks.",
                            "type": "string",
                            "const": "Exception"
                        }
                    ]
                })});

        let self_schema = generator.subschema_for::<Self>();

        json_schema!({
            "title": "MontyObject",
            "description": "A Python value that can be passed to or returned from the Monty interpreter.",
            "oneOf": [
                {
                    "description": "Python's `Ellipsis` singleton (`...`).",
                    "type": "string",
                    "const": "Ellipsis"
                },
                {
                    "description": "Python's `None` singleton.",
                    "type": "string",
                    "const": "None"
                },
                {
                    "description": "Python boolean (`True` or `False`).",
                    "type": "object",
                    "properties": {
                        "Bool": { "type": "boolean" }
                    },
                    "additionalProperties": false,
                    "required": ["Bool"]
                },
                {
                    "description": "Python integer (64-bit signed).",
                    "type": "object",
                    "properties": {
                        "Int": {
                            "type": "integer",
                            "format": "int64"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Int"]
                },
                {
                    "description": "Python arbitrary-precision integer. Serialized as a decimal string in JSON (e.g. `\"123456789012345678901234567890\"`).",
                    "type": "object",
                    "properties": {
                        "BigInt": { "type": "string" }
                    },
                    "additionalProperties": false,
                    "required": ["BigInt"]
                },
                {
                    "description": "Python float (64-bit IEEE 754).",
                    "type": "object",
                    "properties": {
                        "Float": {
                            "type": "number",
                            "format": "double"
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Float"]
                },
                {
                    "description": "Python string (UTF-8).",
                    "type": "object",
                    "properties": {
                        "String": { "type": "string" }
                    },
                    "additionalProperties": false,
                    "required": ["String"]
                },
                {
                    "description": "Python bytes object (sequence of `u8` values).",
                    "type": "object",
                    "properties": {
                        "Bytes": {
                            "type": "array",
                            "items": {
                                "type": "integer",
                                "format": "uint8",
                                "minimum": 0,
                                "maximum": 255
                            }
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Bytes"]
                },
                {
                    "description": "Python list (mutable sequence).",
                    "type": "object",
                    "properties": {
                        "List": {
                            "type": "array",
                            "items": self_schema.clone()
                        }
                    },
                    "additionalProperties": false,
                    "required": ["List"]
                },
                {
                    "description": "Python tuple (immutable sequence).",
                    "type": "object",
                    "properties": {
                        "Tuple": {
                            "type": "array",
                            "items": self_schema.clone()
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Tuple"]
                },
                {
                    "description": "Python named tuple (immutable sequence with named fields). Named tuples behave like tuples but also support attribute access by field name.",
                    "type": "object",
                    "properties": {
                        "NamedTuple": {
                            "type": "object",
                            "properties": {
                                "field_names": {
                                    "description": "Field names in order.",
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "type_name": {
                                    "description": "Type name for repr (e.g. `\"os.stat_result\"`).",
                                    "type": "string"
                                },
                                "values": {
                                    "description": "Values in order (same length as `field_names`).",
                                    "type": "array",
                                    "items": self_schema.clone()
                                }
                            },
                            "required": ["type_name", "field_names", "values"]
                        }
                    },
                    "additionalProperties": false,
                    "required": ["NamedTuple"]
                },
                {
                    "description": "Python dictionary.",
                    "type": "object",
                    "properties": {
                        "Dict": {
                            "type": "array",
                            "items": {
                                "type": "array",
                                "prefixItems": [
                                    self_schema.clone(),
                                    self_schema.clone()
                                ],
                                "items": false,
                                "minItems": 2,
                                "maxItems": 2
                            }
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Dict"]
                },
                {
                    "description": "Python set.",
                    "type": "object",
                    "properties": {
                        "Set": {
                            "type": "array",
                            "items": self_schema.clone()
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Set"]
                },
                {
                    "description": "Python frozenset.",
                    "type": "object",
                    "properties": {
                        "FrozenSet": {
                            "type": "array",
                            "items": self_schema.clone()
                        }
                    },
                    "additionalProperties": false,
                    "required": ["FrozenSet"]
                },
                {
                    "description": "Python exception with type and optional message argument.",
                    "type": "object",
                    "properties": {
                        "Exception": {
                            "type": "object",
                            "properties": {
                                "arg": {
                                    "description": "Optional string argument passed to the exception constructor.",
                                    "type": ["string", "null"]
                                },
                                "exc_type": {
                                    "description": "The exception type (e.g. `ValueError`, `TypeError`).",
                                    "$ref": defs_ref(generator, "MontyExcType")
                                }
                            },
                            "required": ["exc_type"]
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Exception"]
                },
                {
                    "description": "A Python type object (e.g. `int`, `str`, `list`).",
                    "type": "object",
                    "properties": {
                        "Type": {
                            "description": "Represents the Python type of a value (e.g. `int`, `str`, `list`).",
                            "oneOf": [
                                {
                                    "type": "string",
                                    "enum": [
                                        "Ellipsis",
                                        "Type",
                                        "NoneType",
                                        "Bool",
                                        "Int",
                                        "Float",
                                        "Range",
                                        "Slice",
                                        "Str",
                                        "Bytes",
                                        "List",
                                        "Tuple",
                                        "NamedTuple",
                                        "Dict",
                                        "Set",
                                        "FrozenSet",
                                        "Dataclass",
                                        "Function",
                                        "BuiltinFunction",
                                        "Cell",
                                        "Iterator",
                                        "Module"
                                    ]
                                },
                                {
                                    "description": "An exception type — wraps `ExcType`.",
                                    "type": "object",
                                    "properties": {
                                        "Exception": {
                                            "$ref": defs_ref(generator, "MontyExcType")
                                        }
                                    },
                                    "additionalProperties": false,
                                    "required": ["Exception"]
                                },
                                {
                                    "description": "Coroutine type for async functions and external futures.",
                                    "type": "string",
                                    "const": "Coroutine"
                                },
                                {
                                    "description": "Marker type for stdout/stderr.",
                                    "type": "string",
                                    "const": "TextIOWrapper"
                                },
                                {
                                    "description": "typing module special forms (Any, Optional, Union, etc.).",
                                    "type": "string",
                                    "const": "SpecialForm"
                                },
                                {
                                    "description": "A filesystem path from `pathlib.Path`.",
                                    "type": "string",
                                    "const": "Path"
                                },
                                {
                                    "description": "A property descriptor.",
                                    "type": "string",
                                    "const": "Property"
                                }
                            ]
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Type"]
                },
                {
                    "description": "A Python builtin function (e.g. `print`, `len`).",
                    "type": "object",
                    "properties": {
                        "BuiltinFunction": {
                            "description": "Interpreter-native Python builtin functions.",
                            "type": "string",
                            "enum": [
                                "Abs",
                                "All",
                                "Any",
                                "Bin",
                                "Chr",
                                "Divmod",
                                "Enumerate",
                                "Hash",
                                "Hex",
                                "Id",
                                "Isinstance",
                                "Len",
                                "Map",
                                "Max",
                                "Min",
                                "Next",
                                "Oct",
                                "Ord",
                                "Pow",
                                "Print",
                                "Repr",
                                "Reversed",
                                "Round",
                                "Sorted",
                                "Sum",
                                "Type",
                                "Zip"
                            ]
                        }
                    },
                    "additionalProperties": false,
                    "required": ["BuiltinFunction"]
                },
                {
                    "description": "Python `pathlib.Path` object (technically a `PurePosixPath`).",
                    "type": "object",
                    "properties": {
                        "Path": { "type": "string" }
                    },
                    "additionalProperties": false,
                    "required": ["Path"]
                },
                {
                    "description": "A dataclass instance with class name, field names, attributes, and mutability.",
                    "type": "object",
                    "properties": {
                        "Dataclass": {
                            "type": "object",
                            "properties": {
                                "attrs": {
                                    "description": "All attribute name → value mappings (includes fields and extra attrs).",
                                    "type": "array",
                                    "items": {
                                        "type": "array",
                                        "prefixItems": [
                                            self_schema.clone(),
                                            self_schema.clone()
                                        ],
                                        "items": false,
                                        "minItems": 2,
                                        "maxItems": 2
                                    }
                                },
                                "field_names": {
                                    "description": "Declared field names in definition order (for repr).",
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "frozen": {
                                    "description": "Whether this dataclass instance is immutable (`frozen=True`).",
                                    "type": "boolean"
                                },
                                "name": {
                                    "description": "The class name (e.g. `\"Point\"`, `\"User\"`).",
                                    "type": "string"
                                },
                                "type_id": {
                                    "description": "Identifier of the type, from `id(type(dc))` in Python.",
                                    "type": "integer",
                                    "format": "uint64",
                                    "minimum": 0
                                }
                            },
                            "required": ["name", "type_id", "field_names", "attrs", "frozen"]
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Dataclass"]
                },
                {
                    "description": "Fallback for values whose `repr()` string is the best we can do. Output-only — cannot be used as an input to `run`.",
                    "type": "object",
                    "properties": {
                        "Repr": { "type": "string" }
                    },
                    "additionalProperties": false,
                    "required": ["Repr"]
                },
                {
                    "description": "Cycle marker inserted when converting cyclic structures. Output-only — cannot be used as an input to `run`.",
                    "type": "object",
                    "properties": {
                        "Cycle": {
                            "type": "array",
                            "prefixItems": [
                                {
                                    "type": "integer",
                                    "format": "uint",
                                    "minimum": 0
                                },
                                {
                                    "type": "string"
                                }
                            ],
                            "items": false,
                            "minItems": 2,
                            "maxItems": 2
                        }
                    },
                    "additionalProperties": false,
                    "required": ["Cycle"]
                }
            ]
        })
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "MontyObject").into()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "MontyObject".into()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginResourceLimits(pub ResourceLimits);

impl From<PluginResourceLimits> for ResourceLimits {
    fn from(value: PluginResourceLimits) -> Self {
        value.0
    }
}

impl std::ops::Deref for PluginResourceLimits {
    type Target = ResourceLimits;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PluginResourceLimits {
    pub fn into_inner(self) -> ResourceLimits {
        self.0
    }
}

impl JsonSchema for PluginResourceLimits {
    fn inline_schema() -> bool {
        false
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "max_allocations": {
                    "description": "Maximum number of heap allocations allowed.",
                    "type": "integer",
                    "minimum": 0
                },
                "max_duration": {
                    "description": "Maximum execution time.",
                    "type": "object",
                    "properties": {
                        "secs": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "nanos": {
                            "type": "integer",
                            "minimum": 0
                        }
                    },
                    "required": ["secs", "nanos"],
                    "additionalProperties": false
                },
                "max_memory": {
                    "description": "Maximum heap memory in bytes (approximate).",
                    "type": "integer",
                    "minimum": 0
                },
                "gc_interval": {
                    "description": "Run garbage collection every N allocations.",
                    "type": "integer",
                    "minimum": 0
                },
                "max_recursion_depth": {
                    "description": "Maximum recursion depth (function call stack depth).",
                    "type": "integer",
                    "minimum": 0
                }
            },
            "additionalProperties": false
        })
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "ResourceLimits").into()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ResourceLimits".into()
    }
}

/// Input for the `run` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunArguments {
    /// The Python code to execute.
    pub code: String,

    /// Input variables as a map of name to value.
    #[serde(default)]
    pub inputs: HashMap<String, PluginMontyObject>,

    pub resource_limits: Option<PluginResourceLimits>,
}

/// Output returned by the `run` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunResponse {
    /// Captured print/stdout output produced by the Python code.
    pub output: String,

    /// The return value of the Python code (serialized MontyObject).
    pub result: PluginMontyObject,
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema::Validator;
    use monty::{ExcType, MontyObject};
    use serde_json::json;

    fn run_response_validator() -> Validator {
        let schema = schemars::schema_for!(RunResponse);
        let schema_value = serde_json::to_value(&schema).unwrap();
        Validator::new(&schema_value).expect("invalid RunResponse schema")
    }

    // ── PluginMontyObject: From<MontyObject> ────────────────────────

    #[test]
    fn from_monty_object_none() {
        let pmo = PluginMontyObject::from(MontyObject::None);
        assert!(matches!(pmo.0, MontyObject::None));
    }

    #[test]
    fn from_monty_object_int() {
        let pmo = PluginMontyObject::from(MontyObject::Int(42));
        assert!(matches!(pmo.0, MontyObject::Int(42)));
    }

    #[test]
    fn from_monty_object_string() {
        let pmo = PluginMontyObject::from(MontyObject::String("hello".into()));
        assert!(matches!(pmo.0, MontyObject::String(ref s) if s == "hello"));
    }

    #[test]
    fn from_monty_object_bool() {
        let pmo = PluginMontyObject::from(MontyObject::Bool(true));
        assert!(matches!(pmo.0, MontyObject::Bool(true)));
    }

    #[test]
    fn from_monty_object_float() {
        let pmo = PluginMontyObject::from(MontyObject::Float(3.14));
        assert!(matches!(pmo.0, MontyObject::Float(f) if (f - 3.14).abs() < f64::EPSILON));
    }

    #[test]
    fn from_monty_object_bytes() {
        let pmo = PluginMontyObject::from(MontyObject::Bytes(vec![0xCA, 0xFE]));
        assert!(matches!(pmo.0, MontyObject::Bytes(ref b) if b == &[0xCA, 0xFE]));
    }

    #[test]
    fn from_monty_object_list() {
        let pmo = PluginMontyObject::from(MontyObject::List(vec![
            MontyObject::Int(1),
            MontyObject::Int(2),
        ]));
        assert!(matches!(pmo.0, MontyObject::List(ref v) if v.len() == 2));
    }

    #[test]
    fn from_monty_object_exception() {
        let pmo = PluginMontyObject::from(MontyObject::Exception {
            exc_type: ExcType::ValueError,
            arg: Some("bad value".into()),
        });
        assert!(matches!(
            pmo.0,
            MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(ref s),
            } if s == "bad value"
        ));
    }

    // ── PluginMontyObject: From<PluginMontyObject> for MontyObject ──

    #[test]
    fn into_monty_object() {
        let pmo = PluginMontyObject(MontyObject::Int(99));
        let obj: MontyObject = pmo.into();
        assert!(matches!(obj, MontyObject::Int(99)));
    }

    // ── PluginMontyObject: AsRef ────────────────────────────────────

    #[test]
    fn as_ref_returns_inner() {
        let pmo = PluginMontyObject(MontyObject::Bool(false));
        let r: &MontyObject = pmo.as_ref();
        assert!(matches!(r, MontyObject::Bool(false)));
    }

    // ── PluginMontyObject: Deref ────────────────────────────────────

    #[test]
    fn deref_returns_inner() {
        let pmo = PluginMontyObject(MontyObject::String("deref".into()));
        // Deref lets us call MontyObject methods / match directly via &*
        let inner: &MontyObject = &*pmo;
        assert!(matches!(inner, MontyObject::String(s) if s == "deref"));
    }

    // ── PluginMontyObject: into_inner ───────────────────────────────

    #[test]
    fn into_inner_returns_owned() {
        let pmo = PluginMontyObject(MontyObject::Float(2.718));
        let obj = pmo.into_inner();
        assert!(matches!(obj, MontyObject::Float(f) if (f - 2.718).abs() < f64::EPSILON));
    }

    // ── PluginMontyObject: Serde round-trips ────────────────────────

    #[test]
    fn serde_roundtrip_none() {
        let pmo = PluginMontyObject(MontyObject::None);
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::None));
    }

    #[test]
    fn serde_roundtrip_int() {
        let pmo = PluginMontyObject(MontyObject::Int(-123));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::Int(-123)));
    }

    #[test]
    fn serde_roundtrip_string() {
        let pmo = PluginMontyObject(MontyObject::String("café".into()));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::String(ref s) if s == "café"));
    }

    #[test]
    fn serde_roundtrip_bool() {
        let pmo = PluginMontyObject(MontyObject::Bool(true));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::Bool(true)));
    }

    #[test]
    fn serde_roundtrip_float() {
        let pmo = PluginMontyObject(MontyObject::Float(1.5));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::Float(f) if (f - 1.5).abs() < f64::EPSILON));
    }

    #[test]
    fn serde_roundtrip_bytes() {
        let pmo = PluginMontyObject(MontyObject::Bytes(vec![1, 2, 3]));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::Bytes(ref b) if b == &[1, 2, 3]));
    }

    #[test]
    fn serde_roundtrip_list() {
        let pmo = PluginMontyObject(MontyObject::List(vec![
            MontyObject::Int(1),
            MontyObject::String("two".into()),
        ]));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        if let MontyObject::List(items) = &back.0 {
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0], MontyObject::Int(1)));
            assert!(matches!(items[1], MontyObject::String(ref s) if s == "two"));
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn serde_roundtrip_tuple() {
        let pmo = PluginMontyObject(MontyObject::Tuple(vec![
            MontyObject::Bool(false),
            MontyObject::None,
        ]));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        if let MontyObject::Tuple(items) = &back.0 {
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0], MontyObject::Bool(false)));
            assert!(matches!(items[1], MontyObject::None));
        } else {
            panic!("expected Tuple");
        }
    }

    #[test]
    fn serde_roundtrip_dict() {
        let pmo = PluginMontyObject(MontyObject::Dict(
            vec![(MontyObject::String("key".into()), MontyObject::Int(10))].into(),
        ));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::Dict(_)));
    }

    #[test]
    fn serde_roundtrip_exception() {
        let pmo = PluginMontyObject(MontyObject::Exception {
            exc_type: ExcType::TypeError,
            arg: Some("wrong type".into()),
        });
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back.0,
            MontyObject::Exception {
                exc_type: ExcType::TypeError,
                arg: Some(ref s),
            } if s == "wrong type"
        ));
    }

    #[test]
    fn serde_roundtrip_exception_no_arg() {
        let pmo = PluginMontyObject(MontyObject::Exception {
            exc_type: ExcType::RuntimeError,
            arg: None,
        });
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back.0,
            MontyObject::Exception {
                exc_type: ExcType::RuntimeError,
                arg: None,
            }
        ));
    }

    #[test]
    fn serde_roundtrip_nested_structure() {
        let pmo = PluginMontyObject(MontyObject::List(vec![
            MontyObject::Tuple(vec![MontyObject::Int(1), MontyObject::Int(2)]),
            MontyObject::Dict(
                vec![(
                    MontyObject::String("nested".into()),
                    MontyObject::List(vec![MontyObject::Bool(true)]),
                )]
                .into(),
            ),
        ]));
        let json = serde_json::to_string(&pmo).unwrap();
        let back: PluginMontyObject = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.0, MontyObject::List(ref v) if v.len() == 2));
    }

    // ── PluginMontyObject: deserialize from known JSON shapes ───────

    #[test]
    fn deserialize_none_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!("None")).unwrap();
        assert!(matches!(pmo.0, MontyObject::None));
    }

    #[test]
    fn deserialize_int_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!({"Int": 7})).unwrap();
        assert!(matches!(pmo.0, MontyObject::Int(7)));
    }

    #[test]
    fn deserialize_string_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!({"String": "abc"})).unwrap();
        assert!(matches!(pmo.0, MontyObject::String(ref s) if s == "abc"));
    }

    #[test]
    fn deserialize_bool_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!({"Bool": false})).unwrap();
        assert!(matches!(pmo.0, MontyObject::Bool(false)));
    }

    #[test]
    fn deserialize_float_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!({"Float": 0.5})).unwrap();
        assert!(matches!(pmo.0, MontyObject::Float(f) if (f - 0.5).abs() < f64::EPSILON));
    }

    #[test]
    fn deserialize_bytes_from_json() {
        let pmo: PluginMontyObject =
            serde_json::from_value(json!({"Bytes": [255, 0, 128]})).unwrap();
        assert!(matches!(pmo.0, MontyObject::Bytes(ref b) if b == &[255, 0, 128]));
    }

    #[test]
    fn deserialize_ellipsis_from_json() {
        let pmo: PluginMontyObject = serde_json::from_value(json!("Ellipsis")).unwrap();
        assert!(matches!(pmo.0, MontyObject::Ellipsis));
    }

    #[test]
    fn deserialize_invalid_json_fails() {
        let result = serde_json::from_value::<PluginMontyObject>(json!({"Unknown": 1}));
        assert!(result.is_err());
    }

    // ── PluginMontyObject: Clone ────────────────────────────────────

    #[test]
    fn clone_preserves_value() {
        let pmo = PluginMontyObject(MontyObject::Int(77));
        let cloned = pmo.clone();
        assert!(matches!(cloned.0, MontyObject::Int(77)));
    }

    // ── PluginMontyObject: Debug ────────────────────────────────────

    #[test]
    fn debug_format_is_nonempty() {
        let pmo = PluginMontyObject(MontyObject::None);
        let dbg = format!("{:?}", pmo);
        assert!(!dbg.is_empty());
    }

    // ── PluginMontyObject: JsonSchema ───────────────────────────────

    #[test]
    fn json_schema_name() {
        assert_eq!(PluginMontyObject::schema_name(), "MontyObject");
    }

    #[test]
    fn json_schema_id_contains_module_path() {
        let id = PluginMontyObject::schema_id();
        assert!(id.contains("MontyObject"));
    }

    #[test]
    fn json_schema_generates_valid_schema() {
        let schema = schemars::schema_for!(PluginMontyObject);
        let json = serde_json::to_value(&schema).unwrap();
        // The schema should contain a oneOf from the embedded JSON
        assert!(json.is_object());
    }

    // ── RunArguments: Serde ─────────────────────────────────────────

    #[test]
    fn run_arguments_deserialize_code_only() {
        let val = json!({ "code": "print('hi')" });
        let args: RunArguments = serde_json::from_value(val).unwrap();
        assert_eq!(args.code, "print('hi')");
        assert!(args.inputs.is_empty());
    }

    #[test]
    fn run_arguments_deserialize_with_inputs() {
        let val = json!({
            "code": "x + 1",
            "inputs": {
                "x": {"Int": 5}
            }
        });
        let args: RunArguments = serde_json::from_value(val).unwrap();
        assert_eq!(args.code, "x + 1");
        assert_eq!(args.inputs.len(), 1);
        assert!(args.inputs.contains_key("x"));
        assert!(matches!(args.inputs["x"].0, MontyObject::Int(5)));
    }

    #[test]
    fn run_arguments_deserialize_multiple_inputs() {
        let val = json!({
            "code": "a + b",
            "inputs": {
                "a": {"Int": 10},
                "b": {"String": "hello"}
            }
        });
        let args: RunArguments = serde_json::from_value(val).unwrap();
        assert_eq!(args.inputs.len(), 2);
        assert!(matches!(args.inputs["a"].0, MontyObject::Int(10)));
        assert!(matches!(args.inputs["b"].0, MontyObject::String(ref s) if s == "hello"));
    }

    #[test]
    fn run_arguments_deserialize_none_input() {
        let val = json!({
            "code": "x",
            "inputs": {
                "x": "None"
            }
        });
        let args: RunArguments = serde_json::from_value(val).unwrap();
        assert!(matches!(args.inputs["x"].0, MontyObject::None));
    }

    #[test]
    fn run_arguments_missing_code_fails() {
        let val = json!({ "inputs": {} });
        let result = serde_json::from_value::<RunArguments>(val);
        assert!(result.is_err());
    }

    #[test]
    fn run_arguments_roundtrip() {
        let args = RunArguments {
            code: "1 + 2".into(),
            inputs: {
                let mut m = HashMap::new();
                m.insert("x".into(), PluginMontyObject(MontyObject::Int(3)));
                m
            },
            resource_limits: None,
        };
        let json = serde_json::to_value(&args).unwrap();
        let back: RunArguments = serde_json::from_value(json).unwrap();
        assert_eq!(back.code, "1 + 2");
        assert!(matches!(back.inputs["x"].0, MontyObject::Int(3)));
    }

    #[test]
    fn run_arguments_empty_code() {
        let val = json!({ "code": "" });
        let args: RunArguments = serde_json::from_value(val).unwrap();
        assert_eq!(args.code, "");
        assert!(args.inputs.is_empty());
    }

    #[test]
    fn run_arguments_json_schema_generates() {
        let schema = schemars::schema_for!(RunArguments);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json.get("properties").unwrap();
        assert!(props.get("code").is_some());
        assert!(props.get("inputs").is_some());
    }

    // ── RunResponse: Serde ──────────────────────────────────────────

    #[test]
    fn run_response_roundtrip_with_output() {
        let resp = RunResponse {
            output: "hello world\n".into(),
            result: PluginMontyObject(MontyObject::None),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let back: RunResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.output, "hello world\n");
        assert!(matches!(back.result.0, MontyObject::None));
    }

    #[test]
    fn run_response_roundtrip_with_int_result() {
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Int(42)),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let back: RunResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.output, "");
        assert!(matches!(back.result.0, MontyObject::Int(42)));
    }

    #[test]
    fn run_response_roundtrip_with_exception_result() {
        let resp = RunResponse {
            output: "partial output".into(),
            result: PluginMontyObject(MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some("invalid".into()),
            }),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let back: RunResponse = serde_json::from_value(json).unwrap();
        assert_eq!(back.output, "partial output");
        assert!(matches!(
            back.result.0,
            MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some(ref s),
            } if s == "invalid"
        ));
    }

    #[test]
    fn run_response_deserialize_from_json() {
        let val = json!({
            "output": "printed stuff",
            "result": {"Int": 7}
        });
        let resp: RunResponse = serde_json::from_value(val).unwrap();
        assert_eq!(resp.output, "printed stuff");
        assert!(matches!(resp.result.0, MontyObject::Int(7)));
    }

    #[test]
    fn run_response_missing_fields_fails() {
        // Missing result
        let val = json!({ "output": "text" });
        assert!(serde_json::from_value::<RunResponse>(val).is_err());

        // Missing output
        let val = json!({ "result": "None" });
        assert!(serde_json::from_value::<RunResponse>(val).is_err());
    }

    #[test]
    fn run_response_complex_result() {
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Tuple(vec![
                MontyObject::Int(200),
                MontyObject::Dict(
                    vec![(
                        MontyObject::String("content-type".into()),
                        MontyObject::String("text/plain".into()),
                    )]
                    .into(),
                ),
                MontyObject::String("body".into()),
            ])),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let back: RunResponse = serde_json::from_value(json).unwrap();
        if let MontyObject::Tuple(items) = &back.result.0 {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], MontyObject::Int(200)));
            assert!(matches!(items[2], MontyObject::String(ref s) if s == "body"));
        } else {
            panic!("expected Tuple result");
        }
    }

    #[test]
    fn run_response_json_schema_generates() {
        let schema = schemars::schema_for!(RunResponse);
        let json = serde_json::to_value(&schema).unwrap();
        let props = json.get("properties").unwrap();
        assert!(props.get("output").is_some());
        assert!(props.get("result").is_some());
    }

    // ── RunResponse: schema validation ──────────────────────────────

    fn assert_valid(validator: &Validator, value: &serde_json::Value) {
        assert!(
            validator.is_valid(value),
            "expected valid but got errors: {:?}",
            validator.iter_errors(value).collect::<Vec<_>>()
        );
    }

    #[test]
    fn run_response_validates_against_schema_none_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::None),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_int_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: "printed\n".into(),
            result: PluginMontyObject(MontyObject::Int(42)),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_string_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::String("hello".into())),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_bool_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Bool(true)),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_float_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Float(3.14)),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_bytes_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Bytes(vec![0xCA, 0xFE])),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_ellipsis_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Ellipsis),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_exception_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: "partial".into(),
            result: PluginMontyObject(MontyObject::Exception {
                exc_type: ExcType::ValueError,
                arg: Some("bad".into()),
            }),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_exception_no_arg() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Exception {
                exc_type: ExcType::RuntimeError,
                arg: None,
            }),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_list_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::List(vec![
                MontyObject::Int(1),
                MontyObject::String("two".into()),
                MontyObject::Bool(false),
            ])),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_tuple_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Tuple(vec![
                MontyObject::Int(200),
                MontyObject::String("OK".into()),
            ])),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_dict_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::Dict(
                vec![(MontyObject::String("key".into()), MontyObject::Int(10))].into(),
            )),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_complex_nested_result() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: "line 1\nline 2\n".into(),
            result: PluginMontyObject(MontyObject::Tuple(vec![
                MontyObject::Int(200),
                MontyObject::Dict(
                    vec![(
                        MontyObject::String("content-type".into()),
                        MontyObject::String("text/plain".into()),
                    )]
                    .into(),
                ),
                MontyObject::Bytes(vec![72, 101, 108, 108, 111]),
            ])),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_nested_lists() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: String::new(),
            result: PluginMontyObject(MontyObject::List(vec![
                MontyObject::List(vec![MontyObject::Int(1), MontyObject::Int(2)]),
                MontyObject::List(vec![MontyObject::Int(3), MontyObject::Int(4)]),
            ])),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_validates_against_schema_with_output() {
        let validator = run_response_validator();
        let resp = RunResponse {
            output: "hello world\nfoo bar\n".into(),
            result: PluginMontyObject(MontyObject::None),
        };
        assert_valid(&validator, &serde_json::to_value(&resp).unwrap());
    }

    #[test]
    fn run_response_invalid_json_rejected_by_schema() {
        let validator = run_response_validator();

        // Missing "result" field
        assert!(!validator.is_valid(&json!({"output": "text"})));

        // Missing "output" field
        assert!(!validator.is_valid(&json!({"result": "None"})));

        // Wrong type for "output"
        assert!(!validator.is_valid(&json!({"output": 123, "result": "None"})));

        // Completely wrong shape
        assert!(!validator.is_valid(&json!("just a string")));
        assert!(!validator.is_valid(&json!(null)));
    }
}
