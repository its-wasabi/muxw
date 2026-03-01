use std::hash::BuildHasher;

pub struct ConfigField<T> {
    inner: arc_swap::ArcSwap<T>,
}

impl<T> std::fmt::Debug for ConfigField<T>
where
    T: std::fmt::Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let snapshot = self.inner.load_full();
        f.debug_tuple("ConfigField").field(&*snapshot).finish()
    }
}

impl<T: Default + Send + Sync + 'static> Default for ConfigField<T> {
    fn default() -> Self {
        Self {
            inner: arc_swap::ArcSwap::from_pointee(T::default()),
        }
    }
}

impl<T: Send + Sync + 'static> ConfigField<T> {
    pub fn set(&self, data: T) {
        self.inner.store(std::sync::Arc::new(data));
    }

    pub fn get(&self) -> std::sync::Arc<T> {
        self.inner.load_full()
    }
}

impl<T> ConfigField<Vec<T>>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn push(&self, value: T) {
        let current = self.inner.load_full();

        // Clone underlying Vec
        let mut new_vec = (*current).clone();

        new_vec.push(value);

        self.inner.store(std::sync::Arc::new(new_vec));
    }
}

impl<K, V, S: BuildHasher> ConfigField<std::collections::HashMap<K, V, S>>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
{
    pub fn insert(&self, key: K, value: V) {
        let current = self.inner.load_full();

        // clone full map (snapshot model)
        let mut new_map = (*current).clone();

        new_map.insert(key, value);

        self.inner.store(std::sync::Arc::new(new_map));
    }

    pub fn remove(&self, key: &K) {
        let current = self.inner.load_full();
        let mut new_map = (*current).clone();

        new_map.remove(key);

        self.inner.store(std::sync::Arc::new(new_map));
    }

    // pub fn clear(&self) {
    //     self.inner
    //         .store(std::sync::Arc::new(std::collections::HashMap::new()));
    // }
}
