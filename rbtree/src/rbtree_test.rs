use crate::rbtree::RBTree;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_insert_delete_batch(hm in prop::collection::hash_map(".*", ".*", 0..1000)) {
        let mut tree = RBTree::<String, String>::new();
        let mut back = hm.clone();
        for (key, value) in hm {
            assert_eq!(tree.insert(key, value.clone()), None);
        }

        //duplicate insert should return old value
        for (key, value) in &back{
            assert_eq!(tree.insert(key.clone(), value.clone()), Some(value.clone()));
        }

        for (key, value) in &back {
            assert_eq!(tree.get(key).unwrap(), value);
        }

        let mut hm_keys = back.keys().collect::<Vec<&String>>();
        hm_keys.sort();
        assert_eq!(tree.keys().collect::<Vec<&String>>(), hm_keys);

        let half = tree.len()/2;
        for key in back.keys().take(half).map(|s| s.clone()).collect::<Vec<String>>() {
            assert_eq!(tree.remove(&key).unwrap(), *back.remove(&key).unwrap());
        }
        assert_eq!(tree.len(), back.len());

        let half = tree.len()/2;
        for key in tree.keys().take(half).map(|s| s.clone()).collect::<Vec<String>>() {
            assert_eq!(tree.remove(&key).unwrap(), *back.remove(&key).unwrap());
        }
        assert_eq!(tree.len(), back.len());
    }
}
