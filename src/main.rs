use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::process::exit;
use indicatif::ProgressBar;
use nom::{
    combinator::*,
    sequence::tuple,
    IResult,
};
use nom::multi::count;
use nom::number::complete::{le_i32, le_u32, le_u64};
use serde::{Deserialize, Serialize};
use crate::rsz::{parse_rsz, RSZ};
use crate::rsz::json_parser::parse_json;

mod rsz;

#[derive(Serialize, Deserialize)]
struct Prefab {
    magic: u32,
    info_count: i32,
    resource_count: i32,
    gameobject_ref_info_count: i32,
    userdata_count: i32,
    #[serde(skip)]
    reserved: i32,
    gameobject_ref_info_tbl: u64,
    resource_info_tbl: u64,
    userdata_info_tbl: u64,
    data_offset: u64,
}

#[derive(Serialize, Deserialize)]
struct UserData {
    magic: u32,
    resource_count: i32,
    userdata_count: i32,
    info_count: i32,
    resource_info_tbl: u64,
    userdata_info_tbl: u64,
    data_offset: u64,
}

#[derive(Serialize, Deserialize)]
enum HeaderType {
    Prefab(Prefab),
    UserData(UserData),
}

#[derive(Serialize, Deserialize)]
struct PrefabGameObjectInfo {
    id: i32,
    parent_id: i32,
    component_count: i32,
}

#[derive(Serialize, Deserialize)]
struct StandardGameObjectInfo {
    guid: rsz::GUID,
    id: i32,
    parent_id: i32,
    component_count: u16,
    unknown: i16,
    prefab_id: i32,
}

#[derive(Serialize, Deserialize)]
struct UserDataInfo {
    crc: u32,
}

#[derive(Serialize, Deserialize)]
struct RSZFile
{
    header: HeaderType,
    prefab_gameobject_info: Vec<PrefabGameObjectInfo>,
    standard_gameobject_info: Vec<StandardGameObjectInfo>,
}

#[derive(Serialize, Deserialize)]
struct CharacterAssetHeader {
    version: u32,
    #[serde(skip)]
    magic: u32,
    #[serde(skip)]
    id_table_offset: u64,
    #[serde(skip)]
    parent_id_table_offset: u64,
    #[serde(skip)]
    action_list_table_offset: u64,
    #[serde(skip)]
    data_id_table_offset: u64,
    #[serde(skip)]
    data_list_table_offset: u64,
    #[serde(skip)]
    string_object_offset: u64,
    #[serde(skip)]
    string_offset: u64,
    #[serde(skip)]
    object_table_rsz_offset: u64,
    #[serde(skip)]
    object_table_rsz_end: u64,
    #[serde(skip)]
    object_count: u32,
    #[serde(skip)]
    style_count: u32,
    #[serde(skip)]
    data_count: u32,
    #[serde(skip)]
    string_count: u32,
}

fn parse_fchar_header(input: &[u8]) -> IResult<&[u8], CharacterAssetHeader>
{
    map(
        tuple((
            le_u32,
            le_u32,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u32,
            le_u32,
            le_u32,
            le_u32,
            )),
            |(
                 version,
                 magic,
                 id_table_offset,
                 parent_id_table_offset,
                 action_list_table_offset,
                 data_id_table_offset,
                 data_list_table_offset,
                 string_object_offset,
                 string_offset,
                 object_table_rsz_offset,
                 object_table_rsz_end,
                 object_count,
                 style_count,
                 data_count,
                 string_count,
             )|{
                CharacterAssetHeader {
                    version,
                    magic,
                    id_table_offset,
                    parent_id_table_offset,
                    action_list_table_offset,
                    data_id_table_offset,
                    data_list_table_offset,
                    string_object_offset,
                    string_offset,
                    object_table_rsz_offset,
                    object_table_rsz_end,
                    object_count,
                    style_count,
                    data_count,
                    string_count,
                }
            }
    )(input)
}

#[derive(Serialize, Deserialize, Default)]
struct ActionListTable {
    action_list_table_offset: u64,
    style_data_offset: u64,
    action_list_offset: u64,
    action_rsz: u64,
    data_id_table_offset: u64,
    action_list_count: u32,
    object_count: u32,
}

fn parse_action_list_table(input: &[u8]) -> IResult<&[u8], ActionListTable>
{
    map(
        tuple((
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u64,
            le_u32,
            le_u32,
        )),
        |(
             action_list_table_offset,
             style_data_offset,
             action_list_offset,
             action_rsz,
             data_id_table_offset,
             action_list_count,
             object_count,
         )|{
            ActionListTable {
                action_list_table_offset,
                style_data_offset,
                action_list_offset,
                action_rsz,
                data_id_table_offset,
                action_list_count,
                object_count,
            }
        }
    )(input)
}

#[derive(Serialize, Deserialize)]
struct ActionData {
    action_id: i32,
    frames: i32,
    key_start_frame: i32,
    key_end_frame: i32,
}

fn parse_action_data(input: &[u8]) -> IResult<&[u8], ActionData>
{
    map(
        tuple((
            le_i32,
            le_i32,
            le_i32,
            le_i32,
        )),
        |(
             action_id,
             frames,
             key_start_frame,
             key_end_frame,
         )|{
            ActionData {
                action_id,
                frames,
                key_start_frame,
                key_end_frame,
            }
        }
    )(input)
}

#[derive(Serialize, Deserialize)]
pub struct KeyData {
    pub key_start_frame: i32,
    pub key_end_frame: i32,
}

fn parse_key_data(input: &[u8]) -> IResult<&[u8], KeyData>
{
    map(
        tuple((
            le_i32,
            le_i32,
        )),
        |(
             key_start_frame,
             key_end_frame,
         )|{
            KeyData {
                key_start_frame,
                key_end_frame,
            }
        }
    )(input)
}

#[derive(Serialize, Deserialize)]
pub struct ObjectData {
    pub data_count: i32,
    pub reserved: i32,
    pub key_data: Vec<KeyData>
}

fn parse_object_data(input: &[u8]) -> IResult<&[u8], ObjectData>
{
    let (remainder, data_count) = le_i32::<&[u8], nom::error::Error<&[u8]>>(input).unwrap();
    let (remainder, reserved) = le_i32::<&[u8], nom::error::Error<&[u8]>>(remainder).unwrap();
    let (remainder, key_data) = count(parse_key_data, data_count as usize)(remainder).unwrap();
    Ok((remainder, ObjectData {
        data_count,
        reserved,
        key_data,
    }))
}

#[derive(Serialize, Deserialize)]
struct ActionListInfo {
    #[serde(skip)]
    action_offset: u64,
    #[serde(skip)]
    data_start_offset: u64,
    #[serde(skip)]
    rsz_offset: u64,
    #[serde(skip)]
    rsz_end: u64,
    #[serde(skip)]
    action_count: u32,
    #[serde(skip)]
    object_count: u32,
    action_data: ActionData,
}

fn parse_action_list_info(input: &[u8], offset: usize) -> IResult<&[u8], ActionListInfo> {
    let remainder = &input[offset..];
    let (remainder, action_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder).unwrap();
    let action_start = &input[action_offset as usize..];
    let (remainder_new, data_start_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(action_start).unwrap();
    let (remainder_new, rsz_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (remainder_new, rsz_end) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (remainder_new, action_count) = le_u32::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (remainder_new, object_count) = le_u32::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (_, action_data) = parse_action_data(remainder_new).unwrap();
    return Ok((remainder, ActionListInfo{
        action_offset,
        data_start_offset,
        rsz_offset,
        rsz_end,
        action_count,
        object_count,
        action_data
    }))
}

#[derive(Serialize, Deserialize)]
struct ObjectInfo {
    #[serde(skip)]
    object_offset: u64,
    #[serde(skip)]
    data_start_offset: u64,
    #[serde(skip)]
    rsz_offset: u64,
    #[serde(skip)]
    rsz_end: u64,
    pub object_data: ObjectData,
}

fn parse_object_info(input: &[u8], offset: usize) -> IResult<&[u8], ObjectInfo> {
    let remainder = &input[offset..];
    let (remainder, object_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder).unwrap();
    let object_start = &input[object_offset as usize..];
    let (remainder_new, data_start_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(object_start).unwrap();
    let (remainder_new, rsz_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (remainder_new, rsz_end) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder_new).unwrap();
    let (_, object_data) = parse_object_data(remainder_new).unwrap();

    Ok((remainder, ObjectInfo{
        object_offset,
        data_start_offset,
        rsz_offset,
        rsz_end,
        object_data,
    }))
}

#[derive(Serialize, Deserialize)]
struct Object {
    info: ObjectInfo,
    action: RSZ,
}

fn parse_object(input: &[u8], offset: usize) -> IResult<&[u8], Object> {
    let (remainder_new, info) = parse_object_info(input, offset).unwrap();
    let (_, action) = parse_rsz(input, info.rsz_offset.clone() as usize,false).unwrap();

    Ok((remainder_new, Object{
        info,
        action,
    }))
}

#[derive(Serialize, Deserialize)]
struct ActionList {
    info: ActionListInfo,
    action: RSZ,
    objects: Vec<Object>,
}

fn parse_action_list(input: &[u8], offset: usize) -> IResult<&[u8], ActionList> {
    let (_, info) = parse_action_list_info(input, offset).unwrap();
    let (remainder_new, action) = parse_rsz(input, info.rsz_offset.clone() as usize,false).unwrap();
    let mut objects: Vec<Object> = vec![];
    for n in 0..info.object_count.clone() {
        let offset = (info.data_start_offset.clone() + 8 * n as u64) as usize;
        let (_, object) = parse_object(input, offset).unwrap();
        objects.push(object);
    };
    Ok((remainder_new, ActionList{
        info,
        action,
        objects,
    }))
}

#[derive(Serialize, Deserialize)]
struct DataListInfo {
    data_start_offset: u64,
    rsz_offset: u64,
    data_end_offset: u64,
    data_count: u32,
}

fn parse_data_list_info(input: &[u8]) -> IResult<&[u8], DataListInfo>
{
    map(
        tuple((
            le_u64,
            le_u64,
            le_u64,
            le_u32,
        )),
        |(
             data_start_offset,
             rsz_offset,
             data_end_offset,
             data_count,
         )|{
            DataListInfo {
                data_start_offset,
                rsz_offset,
                data_end_offset,
                data_count,
            }
        }
    )(input)
}

#[derive(Serialize, Deserialize)]
struct DataListItem {
    data_list_offset: u64,
    info: DataListInfo,
    data_ids: Vec<u32>,
    data_rsz: RSZ,
}

fn parse_data_list_item(input: &[u8], offset: usize) -> IResult<&[u8], DataListItem> {
    let remainder = &input[offset..];
    let (remainder, data_list_offset) = le_u64::<&[u8], nom::error::Error<&[u8]>>(remainder).unwrap();
    let data_remainder = &input[data_list_offset as usize..];
    let (data_remainder, info) = parse_data_list_info(data_remainder).unwrap();
    let (_, data_ids) = count(le_u32::<&[u8], nom::error::Error<&[u8]>>, info.data_count as usize)(data_remainder).unwrap();
    let (_, data_rsz) = parse_rsz(input, info.rsz_offset.clone() as usize,false).unwrap();
    Ok((remainder, DataListItem{
        data_list_offset,
        info,
        data_ids,
        data_rsz,
    }))
}

#[derive(Serialize, Deserialize)]
struct CharacterAsset {
    header: CharacterAssetHeader,
    #[serde(skip)]
    id_table: Vec<i32>,
    #[serde(skip)]
    parent_id_table: Vec<i32>,
    #[serde(skip)]
    action_list_table: ActionListTable,
    style_data: RSZ,
    action_list: Vec<ActionList>,
    data_id_table: Vec<u32>,
    data_list_table: Vec<DataListItem>,
    personal_data: RSZ,
}

fn parse_fchar(input: &[u8]) -> IResult<&[u8], CharacterAsset> {
    println!("Parsing fchar file...");
    let (remainder, header) = parse_fchar_header(input).unwrap();
    let (remainder, id_table) = count(le_i32::<&[u8], nom::error::Error<&[u8]>>, header.object_count as usize)(remainder).unwrap();
    let (mut remainder, parent_id_table) = count(le_i32::<&[u8], nom::error::Error<&[u8]>>, header.object_count as usize)(remainder).unwrap();
    let alignment_remainder = (16 - (input.len() - remainder.len()) % 16) % 16;
    if alignment_remainder != 0 {
        remainder = &remainder[alignment_remainder..];
    }
    let (mut remainder, action_list_table) = parse_action_list_table(remainder).unwrap();
    println!("Header parsed!");
    println!("Parsing style data...");
    let (_, style_data) = parse_rsz(input, (&action_list_table.action_rsz + &action_list_table.style_data_offset) as usize, true).unwrap();
    println!("Style data parsed!");
    let mut action_list: Vec<ActionList> = vec![];
    println!("Parsing action list...");
    let bar = ProgressBar::new(action_list_table.action_list_count.clone() as u64);
    for _ in 0..action_list_table.action_list_count {
        let offset = input.len() - remainder.len();
        let (_, action) = parse_action_list(input, offset).unwrap();
        action_list.push(action);
        remainder = &remainder[8..];
        bar.inc(1);
    }
    bar.finish();
    println!("Action list parsed!");
    
    println!("Parsing data tables...");
    let data_id_remainder = &input[header.data_id_table_offset.clone() as usize..];
    let (mut data_remainder, data_id_table) = count(le_u32::<&[u8], nom::error::Error<&[u8]>>, header.data_count as usize)(data_id_remainder).unwrap();
    let mut data_list_remainder = &input[header.data_list_table_offset.clone() as usize..];
    let mut data_list_table: Vec<DataListItem> = vec![];
    for _ in 0..header.data_count {
        let offset = input.len() - data_list_remainder.len();
        let (remainder_new, data_list_item) = parse_data_list_item(input, offset).unwrap();
        data_list_remainder = remainder_new;
        data_list_table.push(data_list_item);
    }
    println!("Data tables parsed!");
    
    println!("Parsing personal data...");
    let (_, personal_data) = parse_rsz(input, header.object_table_rsz_offset.clone() as usize, false).unwrap();
    println!("Personal data parsed!");

    println!("Fchar file parsed!");

    Ok((input, CharacterAsset {
        header,
        id_table,
        parent_id_table,
        action_list_table,
        style_data,
        action_list,
        data_id_table,
        data_list_table,
        personal_data,
    }))
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 3 {
        println!("\nNot enough arguments! First argument should be the file to parse.\nSecond argument should be the RSZ json dump.\nThird argument should be the output json.");
        exit(1)
    }
    
    parse_json(args[2].clone())?;
    
    let mut reader = BufReader::with_capacity(0x3fffff,File::open(&args[1]).unwrap());
    let mut buffer: Vec<u8> = vec![];
    reader.read_to_end(&mut buffer).unwrap();
    
    let fchar_file = parse_fchar(&buffer).unwrap().1;
    let serialized_fchar = serde_json::to_string_pretty(&fchar_file).unwrap();
    println!("Writing fchar to json...");

    std::fs::write(&args[3], serialized_fchar)?;
    println!("Complete!");

    Ok(())
}