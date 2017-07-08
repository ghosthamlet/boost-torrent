use std::str;
use error::{BoostError, BoostResult};
///BencodeValue, one of int, string, list, dictionary.
pub enum BencodeValue<'a> {
    Integer(i32),
    Str(&'a [u8]),
    List(Vec<BencodeValue<'a>>),
    Dict(Vec<(&'a [u8],BencodeValue<'a>)>)
}

impl<'a> BencodeValue<'a> {

    ///Creates a new BencodeValue from the given u8 slice.
    pub fn bdecode(data: &'a [u8]) -> BoostResult<Self> {
        let (res, _) = bdec(data);
        res
    }

    pub fn bencode(&self) -> Vec<u8> {
        match self {
            &BencodeValue::Integer(i) => {
                let mut res = Vec::new();
                res.push('i' as u8);
                res.extend_from_slice(i.to_string().as_bytes());
                res.push('e' as u8);
                res
            },

            &BencodeValue::Str(ref s) => {
                let mut res = Vec::new();
                res.extend_from_slice((s.len().to_string()+":").as_bytes());
                res.extend_from_slice(s);
                res
            },
            &BencodeValue::List(ref l) => {
                let mut res = Vec::new();
                res.push('l' as u8);
                for val in l {
                    res.append(&mut val.bencode());
                }
                res.push('e' as u8);
                res
            },
            &BencodeValue::Dict(ref d) => {
                let mut res = Vec::new();
                res.push('d' as u8);
                for &(ref s,ref v) in d {
                    res.extend_from_slice((s.len().to_string() + ":").as_bytes());
                    res.extend_from_slice(s);
                    res.append(&mut v.bencode());

                }
                res.push('e' as u8);
                res

            }
        }
    }
}


///parses a bencoded data string, returns a result and the index after the last character parsed
fn bdec<'a>(data: &'a [u8]) -> (BoostResult<BencodeValue<'a>>, usize) {
    if data[0] as char == 'd' {
        //list
        let mut pos = 1;
        let mut dct = Vec::new();
        //loop while first unparsed part of dict is e
        while data[pos] as char != 'e' {

            match bdec(&data[pos .. data.len()]) {
                //parse string as key, if ok, parse value
                (Ok(BencodeValue::Str(string)), end) => {
                    //if parse value ok, advance position
                    pos += end;
                    match bdec(&data[pos .. data.len()]) {
                        (Ok(val), end) => {
                            dct.push((string,val));
                            pos += end;
                        },
                        (err, _) => return (err, 0)
                    }
                },
                //if recursion has err, just return err
                (err, _) => return (err, 0)
            }
        }
        (Ok(BencodeValue::Dict(dct)), pos+1)

    } else if data[0] as char == 'i' {
        //integer
        //find end
        if let Some((int_end,_)) = data.iter().enumerate().find(|&r| *r.1 as char == 'e') {
            //parse from after i to before e
            let int_data = &data[1 .. int_end];
            if let Ok(int_str) = str::from_utf8(int_data) {
                match int_str.parse::<i32>() {
                    Ok(int) => (Ok(BencodeValue::Integer(int)),int_end+1),
                    Err(_) => (Err(BoostError::BencodeDecodingErr),0)
                }
            } else {
                (Err(BoostError::BencodeDecodingErr),0)
            }
        } else {
            (Err(BoostError::BencodeDecodingErr),0)
        }


    } else if data[0] as char == 'l' {
        //list
        let mut pos = 1;
        let mut lst = Vec::new();
        //loop while first unparsed part of list is e
        while data[pos] as char != 'e' {
            match bdec(&data[pos .. data.len()]) {
                //if result is ok, push result and advance position
                (Ok(val), end) => {
                    lst.push(val);
                    pos += end;
                },
                //if recursion has err, just return err
                (err, _) => return (err, 0)
            }
        }
        (Ok(BencodeValue::List(lst)), pos+1)

    } else {
        //string
        //find colon
        if let Some((idx,_)) = data.iter().enumerate().find(|&r| *r.1 as char == ':') {
            let (blen, string) = data.split_at(idx);
            //parse len
            if let Ok(len) = str::from_utf8(blen) {
                if let Ok(slen) = len.parse::<usize>() {
                    (Ok(BencodeValue::Str(&string[1 .. slen+1])), (len.len() + slen+1) )
                } else {
                    (Err(BoostError::BencodeDecodingErr),0)
                }
            } else {
                (Err(BoostError::BencodeDecodingErr),0)
            }
        } else {
            (Err(BoostError::BencodeDecodingErr),0)
        }

    }
}
