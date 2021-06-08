use rnest::Provider;

#[derive(Provider)]
pub struct UserStore {}

impl UserStore {
    pub async fn get_list(&self) -> Vec<(u32, String)> {
        return vec![(1, "bob".to_string()), (1, "alice".to_string())];
    }
}
