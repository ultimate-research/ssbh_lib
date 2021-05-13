#[cfg(test)]
mod tests {
    use ssbh_lib::{RelPtr64, SsbhString, SsbhString8};

    #[test]
    fn serialize_deserialize_relptr64_none() {
        let text = serde_json::to_string(&RelPtr64::<u8>::null()).unwrap();
        assert_eq!("null", text);

        let v: RelPtr64<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(None, (*v));
    }

    #[test]
    fn serialize_deserialize_relptr64_some() {
        let text = serde_json::to_string(&RelPtr64::<u8>::new(5)).unwrap();
        assert_eq!("5", text);

        let v: RelPtr64<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(5, (*v).unwrap());
    }

    #[test]
    fn serialize_deserialize_ssbh_string() {
        let v: SsbhString = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!("abc", v.get_string().unwrap());
    }

    #[test]
    fn serialize_deserialize_ssbh_string_null() {
        let v: SsbhString = serde_json::from_str("null").unwrap();
        assert_eq!(None, v.get_string());
    }

    #[test]
    fn serialize_deserialize_ssbh_string_empty() {
        let v: SsbhString = serde_json::from_str("\"\"").unwrap();
        assert_eq!("", v.get_string().unwrap());
    }

    #[test]
    fn serialize_deserialize_ssbh_string8() {
        let v: SsbhString8 = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!("abc", v.get_string().unwrap());
    }

    #[test]
    fn serialize_deserialize_ssbh_string8_null() {
        let v: SsbhString8 = serde_json::from_str("null").unwrap();
        assert_eq!(None, v.get_string());
    }

    #[test]
    fn serialize_deserialize_ssbh_string8_empty() {
        let v: SsbhString8 = serde_json::from_str("\"\"").unwrap();
        assert_eq!("", v.get_string().unwrap());
    }
}
