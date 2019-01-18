//!
//! A CAN database (dbc) format parser written with Rust's nom parser combinator library.
//! CAN databases are used to exchange details about a CAN network.
//! E.g. what messages are being send over the CAN bus and what data do they contain.
//!
//! ```rust
//! use can_dbc::DBC;
//! use codegen::Scope;
//!
//! use std::fs::File;
//! use std::io;
//! use std::io::prelude::*;
//!
//! fn main() -> io::Result<()> {
//!     let mut f = File::open("./examples/sample.dbc")?;
//!     let mut buffer = Vec::new();
//!     f.read_to_end(&mut buffer)?;
//!
//!     let dbc = can_dbc::DBC::from_slice(&buffer).expect("Failed to parse dbc file");
//!
//!     let mut scope = Scope::new();
//!     for message in dbc.messages() {
//!         for signal in message.signals() {
//!
//!             let mut scope = Scope::new();
//!             let message_struct = scope.new_struct(message.message_name());
//!             for signal in message.signals() {
//!                 message_struct.field(signal.name().to_lowercase().as_str(), "f64");
//!             }
//!         }
//!     }
//!
//!     println!("{}", scope.to_string());
//!     Ok(())
//! }
//! ```

#[cfg(feature = "with-serde")]
extern crate serde;
#[cfg(feature = "with-serde")]
#[macro_use]
extern crate serde_derive;

use derive_getters::Getters;
use nom::*;
use nom::types::CompleteByteSlice;

pub mod parser;

#[cfg(test)]
mod tests {

    use super::*;
    use std::str;

    #[test]
    fn dbc_definition_test() {
        let sample_dbc =
        b"
VERSION \"0.1\"
NS_ :
    NS_DESC_
    CM_
    BA_DEF_
    BA_
    VAL_
    CAT_DEF_
    CAT_
    FILTER
    BA_DEF_DEF_
    EV_DATA_
    ENVVAR_DATA_
    SGTYPE_
    SGTYPE_VAL_
    BA_DEF_SGTYPE_
    BA_SGTYPE_
    SIG_TYPE_REF_
    VAL_TABLE_
    SIG_GROUP_
    SIG_VALTYPE_
    SIGTYPE_VALTYPE_
    BO_TX_BU_
    BA_DEF_REL_
    BA_REL_
    BA_DEF_DEF_REL_
    BU_SG_REL_
    BU_EV_REL_
    BU_BO_REL_
    SG_MUL_VAL_
BS_:
BU_: PC
BO_ 2000 WebData_2000: 4 Vector__XXX
    SG_ Signal_8 : 24|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_7 : 16|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_6 : 8|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_5 : 0|8@1+ (1,0) [0|255] \"\" Vector__XXX
BO_ 1840 WebData_1840: 4 PC
    SG_ Signal_4 : 24|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_3 : 16|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_2 : 8|8@1+ (1,0) [0|255] \"\" Vector__XXX
    SG_ Signal_1 : 0|8@1+ (1,0) [0|0] \"\" Vector__XXX

EV_ Environment1: 0 [0|220] \"\" 0 6 DUMMY_NODE_VECTOR0 DUMMY_NODE_VECTOR2;
EV_ Environment2: 0 [0|177] \"\" 0 7 DUMMY_NODE_VECTOR1 DUMMY_NODE_VECTOR2;
ENVVAR_DATA_ SomeEnvVarData: 399;

CM_ SG_ 4 TestSigLittleUnsigned1 \"asaklfjlsdfjlsdfgls
HH?=(%)/&KKDKFSDKFKDFKSDFKSDFNKCnvsdcvsvxkcv\";
CM_ SG_ 5 TestSigLittleUnsigned1 \"asaklfjlsdfjlsdfgls
=0943503450KFSDKFKDFKSDFKSDFNKCnvsdcvsvxkcv\";

BA_DEF_DEF_ \"BusType\" \"AS\";

BA_ \"Attr\" BO_ 4358435 283;
BA_ \"Attr\" BO_ 56949545 344;
";
        match DBC::from_slice(sample_dbc) {
            Ok(dbc_content) => println!("DBC Content{:#?}", dbc_content),
            Err(e) => {
                match e {
                    Error::NomError(nom::Err::Incomplete(needed)) => eprintln!("Error incomplete input, needed: {:?}", needed),
                    Error::NomError(nom::Err::Error(ctx)) => {
                        match ctx {
                            verbose_errors::Context::Code(i, kind) => eprintln!("Error Kind: {:?}, Code: {:?}", kind, str::from_utf8(i.as_bytes())),
                            verbose_errors::Context::List(l)=> eprintln!("Error List: {:?}", l),
                        }
                    }
                    Error::NomError(nom::Err::Failure(ctx)) => eprintln!("Failure {:?}", ctx),
                    Error::Incomplete(dbc, remaining) => eprintln!("Not all data in buffer was read {:#?}, remaining unparsed: {}", dbc, String::from_utf8(remaining).unwrap())
                }
                panic!("Failed to read DBC");
            }
        }
    }
}

/// Possible error cases for `can-dbc`
#[derive(Debug)]
pub enum Error<'a> {
    // Remaining String
    Incomplete(DBC, Vec<u8>),
    NomError(nom::Err<nom::types::CompleteByteSlice<'a>, u32>)
}

/// Baudrate of network in kbit/s
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Baudrate(u64);

/// One or multiple signals are the payload of a CAN frame.
/// To determine the actual value of a signal the following fn applies:
/// `let fnvalue = |can_signal_value| -> can_signal_value * factor + offset;`
#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Signal {
    name: String,
    multiplexer_indicator: MultiplexIndicator,
    pub start_bit: u64,
    pub signal_size: u64,
    byte_order: ByteOrder,
    value_type: ValueType,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    unit: String,
    receivers: Vec<String>,
}

/// CAN id in header of CAN frame.
/// Must be unique in DBC file.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MessageId(pub u64);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Transmitter {
    /// node transmitting the message
    NodeName(String),
    /// message has no sender
    VectorXXX
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MessageTransmitter {
    message_id: MessageId,
    transmitter: Vec<Transmitter>,
}

/// Version generated by DB editor
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Version(String);

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Symbol(String);

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum MultiplexIndicator {
    /// Multiplexor switch
    Multiplexor,
    /// Signal us being multiplexed by the multiplexer switch.
    MultiplexedSignal(u64),
    /// Normal signal
    Plain,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ValueType {
    Signed,
    Unsigned,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum EnvType {
    EnvTypeFloat,
    EnvTypeu64,
    EnvTypeData,
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SignalType {
    signal_type_name: String,
    signal_size: u64,
    byte_order: ByteOrder,
    value_type: ValueType,
    factor: f64,
    offset: f64,
    min: f64,
    max: f64,
    unit: String,
    default_value: f64,
    value_table: String,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AccessType {
    DummyNodeVector0,
    DummyNodeVector1,
    DummyNodeVector2,
    DummyNodeVector3,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AccessNode {
    AccessNodeVectorXXX,
    AccessNodeName(String),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SignalAttributeValue {
    Text(String),
    Int(i64),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AttributeValuedForObjectType {
    RawAttributeValue(AttributeValue),
    NetworkNodeAttributeValue(String, AttributeValue),
    MessageDefinitionAttributeValue(MessageId, Option<AttributeValue>),
    SignalAttributeValue(MessageId, String, AttributeValue),
    EnvVariableAttributeValue(String, AttributeValue),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AttributeValueType {
    AttributeValueTypeInt(i64, i64),
    AttributeValueTypeHex(i64, i64),
    AttributeValueTypeFloat(f64, f64),
    AttributeValueTypeString,
    AttributeValueTypeEnum(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ValDescription {
    a: f64,
    b: String,
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AttrDefault {
    name: String,
    value: AttributeValue,
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum AttributeValue {
    AttributeValueU64(u64),
    AttributeValueI64(i64),
    AttributeValueF64(f64),
    AttributeValueCharString(String),
}

/// Global value table
#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ValueTable {
    value_table_name: String,
    value_descriptions: Vec<ValDescription>
}

/// Object comments
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Comment {
    Node { node_name: String, comment: String },
    Message { message_id: MessageId, comment: String },
    Signal { message_id: MessageId, signal_name: String, comment: String },
    EnvVar { env_var_name: String, comment: String },
    Plain { comment: String },
}

/// CAN message (frame) details including signal details
#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Message {
    /// CAN id in header of CAN frame.
    /// Must be unique in DBC file.
    message_id: MessageId,
    message_name: String,
    message_size: u64,
    transmitter: Transmitter,
    signals: Vec<Signal>
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EnvironmentVariable {
    env_var_name: String,
    env_var_type: EnvType,
    min: i64,
    max: i64,
    unit: String,
    initial_value: f64,
    ev_id: i64,
    access_type: AccessType,
    access_nodes: Vec<AccessNode>,
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EnvironmentVariableData {
    env_var_name: String,
    data_size: u64,
}

/// CAN network nodes, names must be unique
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Node(Vec<String>);

#[derive(Clone,Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AttributeDefault {
    attribute_name: String,
    attribute_value: AttributeValue,
}

#[derive(Clone,Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AttributeValueForObject {
    attribute_name: String,
    attribute_value: AttributeValuedForObjectType,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum AttributeDefinition {
    // TODO add properties
    Message(String),
    // TODO add properties
    Node(String),
    // TODO add properties
    Signal(String),
    EnvironmentVariable(String),
    // TODO figure out name
    Plain(String),
}

/// Encoding for signal raw values.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ValueDescription {
    Signal {
        message_id: MessageId,
        signal_name: String,
        value_descriptions: Vec<ValDescription>
    },
    EnvironmentVariable {
        env_var_name: String,
        value_descriptions: Vec<ValDescription>
    },
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SignalTypeRef {
    message_id: MessageId,
    signal_name: String,
    signal_type_name: String,
}

/// Signal groups define a group of signals within a message
#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SignalGroups {
    message_id: MessageId,
    signal_group_name: String,
    repetitions: u64,
    signal_names: Vec<String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum SignalExtendedValueType {
    SignedOrUnsignedInteger,
    IEEEfloat32Bit,
    IEEEdouble64bit,
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SignalExtendedValueTypeList {
    message_id: MessageId,
    signal_name: String,
    signal_extended_value_type: SignalExtendedValueType
}

#[derive(Clone, Debug, PartialEq, Getters)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DBC {
    /// Version generated by DB editor
    version: Version,
    new_symbols: Vec<Symbol>,
    /// Baud rate of network
    bit_timing: Option<Vec<Baudrate>>,
    /// CAN network nodes
    nodes: Vec<Node>,
    /// Global value table
    value_tables: Vec<ValueTable>,
    /// CAN message (frame) details including signal details
    messages: Vec<Message>,
    message_transmitters: Vec<MessageTransmitter>,
    environment_variables: Vec<EnvironmentVariable>,
    environment_variable_data: Vec<EnvironmentVariableData>,
    signal_types: Vec<SignalType>,
    /// Object comments
    comments: Vec<Comment>,
    attribute_definitions: Vec<AttributeDefinition>,
    // undefined
    // sigtype_attr_list: SigtypeAttrList,
    attribute_defaults: Vec<AttributeDefault>,
    attribute_values: Vec<AttributeValueForObject>,
    /// Encoding for signal raw values
    value_descriptions: Vec<ValueDescription>,
    // obsolete + undefined
    // category_definitions: Vec<CategoryDefinition>,
    // obsolete + undefined
    //categories: Vec<Category>,
    // obsolete + undefined
    //filter: Vec<Filter>,
    signal_type_refs: Vec<SignalTypeRef>,
    /// Signal groups define a group of signals within a message
    signal_groups: Vec<SignalGroups>,
    signal_extended_value_type_list: Option<SignalExtendedValueTypeList>,
}

impl DBC {
    pub fn from_slice(buffer: &[u8]) -> Result<DBC, Error> {
        match parser::dbc(CompleteByteSlice(buffer)) {
            Ok((remaining, dbc)) => {
                if !remaining.is_empty() {
                    return Err(Error::Incomplete(dbc, remaining.as_bytes().to_vec()));
                }
                Ok(dbc)
            },
            Err(e) => Err(Error::NomError(e))
        }
    }

    /// Lookup a message comment
    pub fn message_comment(&self, message_id: &MessageId) -> Option<&str> {
        self.comments
        .iter()
        .filter_map(|x| {
            match x {
                Comment::Message { message_id: ref x_message_id, ref comment } => {
                    if x_message_id == message_id {
                        Some(comment.as_str())
                    } else {
                        None
                    }
                },
                _ => None
            }
        }).next()
    }

    /// Lookup a signal comment
    pub fn signal_comment(&self, message_id: &MessageId, signal_name: &str) -> Option<&str> {
        self.comments
        .iter()
        .filter_map(|x| {
            match x {
                Comment::Signal { message_id: ref x_message_id, signal_name: ref x_signal_name, comment } => {
                    if x_message_id == message_id && x_signal_name == signal_name {
                        Some(comment.as_str())
                    } else {
                        None
                    }
                },
                _ => None
            }
        }).next()
    }

    /// Lookup value descriptions for signal
    pub fn value_descriptions_for_signal(&self, message_id: &MessageId, signal_name: &str) -> Option<&[ValDescription]> {
        self.value_descriptions
            .iter()
            .filter_map(|x| {
                match x {
                    ValueDescription::Signal { message_id: ref x_message_id, signal_name: ref x_signal_name, ref value_descriptions} => {
                        if x_message_id == message_id && x_signal_name == signal_name {
                            Some(value_descriptions.as_slice())
                        } else {
                            None
                        }
                    },
                    _ => None
                }
            }).next()
    }
}