use std::str;
///BencodeValue, one of int, string, list, dictionary.
pub enum BencodeValue<'a> {
    Integer(i32),
    Str(&'a [u8]),
    List(Vec<BencodeValue<'a>>),
    Dict(Vec<(&'a [u8],BencodeValue<'a>)>)
}

pub fn bdecode<'a, 'b>(data: &'a [u8]) -> Result<BencodeValue<'a>, &'b str> {
    let (res, _) = bdec(data);
    res
}

///parses a bencoded data string, returns a result and the index after the last character parsed
fn bdec<'a, 'b>(data: &'a [u8]) -> (Result<BencodeValue<'a>, &'b str>, usize) {
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
                    Err(_) => (Err("Error parsing an integer"),0)
                }
            } else {
                (Err("Error parsing integer, could not read ascii digits"),0)
            }
        } else {
            (Err("Error parsing integer, could not find end"),0)
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
                    (Err("Error parsing string, Could not determine length"),0)
                }
            } else {
                (Err("Error parsing string, could not read ascii length"),0)
            }
        } else {
            (Err("Error parsing string, no separating colon found"),0)
        }

    }
}
