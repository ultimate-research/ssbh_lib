#[cfg(test)]
mod tests {
    use ssbh_lib::{Ptr64, RelPtr64, SsbhArray, SsbhString, SsbhString8};

    #[test]
    fn serialize_deserialize_ssbh_array_empty() {
        let text = serde_json::to_string(&SsbhArray::<u8>::new(Vec::new())).unwrap();
        assert_eq!("[]", text);

        let v: SsbhArray<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(SsbhArray::<u8>::new(Vec::new()), v);
    }

    #[test]
    fn serialize_deserialize_ssbh_array() {
        let text = serde_json::to_string(&SsbhArray::<u8>::new(vec![1, 2, 3])).unwrap();
        assert_eq!("[1,2,3]", text);

        let v: SsbhArray<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(SsbhArray::<u8>::new(vec![1, 2, 3]), v);
    }

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
    fn serialize_deserialize_ptr64_some() {
        let text = serde_json::to_string(&Ptr64::<u8>::new(5)).unwrap();
        assert_eq!("5", text);

        let v: Ptr64<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(5, (*v).unwrap());
    }

    #[test]
    fn serialize_deserialize_ptr64_none() {
        let text = serde_json::to_string(&Ptr64::<u8>::null()).unwrap();
        assert_eq!("null", text);

        let v: Ptr64<u8> = serde_json::from_str(&text).unwrap();
        assert_eq!(None, (*v));
    }

    #[test]
    fn deserialize_ssbh_string() {
        let v: SsbhString = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!("abc", v.to_str().unwrap());
    }

    #[test]
    fn deserialize_ssbh_string_null() {
        let v: SsbhString = serde_json::from_str("null").unwrap();
        assert_eq!(None, v.to_str());
    }

    #[test]
    fn deserialize_ssbh_string_empty() {
        let v: SsbhString = serde_json::from_str("\"\"").unwrap();
        assert_eq!("", v.to_str().unwrap());
    }

    #[test]
    fn deserialize_ssbh_string8() {
        let v: SsbhString8 = serde_json::from_str("\"abc\"").unwrap();
        assert_eq!("abc", v.to_str().unwrap());
    }

    #[test]
    fn deserialize_ssbh_string8_null() {
        let v: SsbhString8 = serde_json::from_str("null").unwrap();
        assert_eq!(None, v.to_str());
    }

    #[test]
    fn deserialize_ssbh_string8_empty() {
        let v: SsbhString8 = serde_json::from_str("\"\"").unwrap();
        assert_eq!("", v.to_str().unwrap());
    }
}
