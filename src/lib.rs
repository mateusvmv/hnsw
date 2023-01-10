use rand::random;
use std::{
	collections::{btree_map::Entry, BTreeMap, HashMap},
	hash::{BuildHasher, Hash, Hasher},
};
use strsim::sorensen_dice;
fn dice(a: &str, b: &str) -> usize {
	((1.0 - sorensen_dice(a, b)) * usize::MAX as f64) as usize
}
struct Layer {
	items: Vec<String>,
	links: HashMap<usize, Vec<usize>>,
	index: HashMap<u64, usize>,
}
impl Layer {
	fn new() -> Self {
		Layer {
			items: vec![],
			links: HashMap::new(),
			index: HashMap::new(),
		}
	}
	fn hash(&self, s: &str) -> u64 {
		let mut h = self.index.hasher().build_hasher();
		s.hash(&mut h);
		h.finish()
	}
	fn index_of(&self, s: &str) -> Option<usize> {
		self.index.get(&self.hash(s)).copied()
	}
	fn push(&mut self, s: &str) -> usize {
		let i = self.items.len();
		self.items.push(s.to_owned());
		self.index.insert(self.hash(s), i);
		i
	}
	fn all<'l>(&'l self, query: &str) -> Vec<(&'l str, usize)> {
		self.items
			.iter()
			.map(|s| (s.as_ref(), dice(s, query)))
			.collect()
	}
	fn nearest<'s>(
		&'s self,
		query: &str,
		entries: &[(&str, usize)],
		k: usize,
	) -> Vec<(&'s str, usize)> {
		let mut queue = BTreeMap::new();
		let mut dists = BTreeMap::new();
		let default = match entries.is_empty() {
			true => self.all(query),
			false => vec![],
		};
		let entries = entries.iter().chain(&default);
		for &(e, d) in entries {
			let i = match self.index_of(e) {
				Some(i) => i,
				None => continue,
			};
			let s = self.items[i].as_str();
			queue.insert(d, s);
			dists.insert(s, d);
		}
		while let Some((d, e)) = queue.pop_first() {
			let i = match self.index_of(e) {
				Some(i) => i,
				None => continue,
			};
			for &l in self.links.get(&i).map(|v| v.as_slice()).unwrap_or_default() {
				let s = self.items[l].as_str();
				let nd = match dists.entry(s) {
					Entry::Vacant(e) => e.insert(dice(query, s)),
					Entry::Occupied(_) => continue,
				};
				if *nd > d {
					continue;
				}
				queue.insert(*nd, s);
			}
		}
		let mut dists: Vec<(&str, usize)> = dists.into_iter().collect();
		dists.sort_by_key(|e| e.1);
		dists.into_iter().take(k).collect()
	}
	fn insert<'s>(
		&'s mut self,
		s: &str,
		entries: &[(&str, usize)],
		k: usize,
	) -> Vec<(&str, usize)> {
		let i = self.push(s);
		let nearest: Vec<_> = self
			.nearest(s, entries, k)
			.into_iter()
			.map(|(s, d)| (self.hash(s), d))
			.collect();
		let js: Vec<usize> = nearest
			.iter()
			.map(|(h, _)| &self.index[h])
			.copied()
			.collect();
		for &j in &js {
			self.links.entry(i).or_default().push(j);
			self.links.entry(j).or_default().push(i);
		}
		nearest
			.into_iter()
			.map(|(h, d)| (self.items[self.index[&h]].as_str(), d))
			.collect()
	}
}
pub struct Hnsw {
	layers: Vec<Layer>,
}
impl Default for Hnsw {
	fn default() -> Self {
		let mut r = Hnsw { layers: Vec::new() };
		r.layers.resize_with(10, Layer::new);
		r
	}
}
impl Hnsw {
	pub fn new() -> Self {
		Self::default()
	}
	fn entry(&self, query: &str) -> Vec<(&str, usize)> {
		self.layers.last().unwrap().all(query)
	}
	pub fn insert(&mut self, s: &str) {
		let mut entries = self.entry(s);
		let n = (-random::<f64>().ln() * 0.6) as usize % 10;
		for i in (0..=n).rev() {
			let l = unsafe {
				let ptr = (&self.layers[i]) as *const Layer as *mut Layer;
				&mut *ptr
			};
			let links = match i {
				0 => 50,
				_ => 5,
			};
			entries = l.insert(s, &entries, links).into_iter().collect();
		}
	}
	pub fn nearest<'s>(&'s self, s: &str, k: usize) -> Vec<(&'s str, usize)> {
		let mut entries = self.entry(s);
		let mut dists: BTreeMap<_, _> = entries.iter().map(|&(s, d)| (d, s)).collect();
		for i in (0..10).rev() {
			let l = unsafe {
				let ptr = (&self.layers[i]) as *const Layer as *mut Layer;
				&mut *ptr
			};
			let links = match i {
				0 => 50,
				_ => 5,
			};
			entries = l.nearest(s, &entries, links);
			for &(s, d) in &entries {
				dists.insert(d, s);
			}
		}
		dists.into_iter().take(k).map(|(d, s)| (s, d)).collect()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn it() {
		let mut h = Hnsw::new();
		for fruit in ["apple", "orange", "banana", "lemon"] {
			h.insert(fruit);
		}
		for fruit in ["apple", "orange", "banana", "lemon"] {
			assert_eq!(fruit, h.nearest(fruit, 1)[0].0);
		}
	}
}