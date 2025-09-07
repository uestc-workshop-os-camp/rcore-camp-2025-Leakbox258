//! Dead lock detect

use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;

///
pub trait DetectInfo {
    ///
    fn get_allocation(&self) -> BTreeMap<usize, usize>;
    ///
    fn get_need(&self) -> BTreeMap<usize, usize>;
}

///
pub trait DeadLockDetect {
    ///
    fn get_available(&self) -> Vec<usize>;
    ///
    fn get_allocation(&self) -> BTreeMap<usize, Vec<usize>>;
    ///
    fn get_need(&self) -> BTreeMap<usize, Vec<usize>>;
    /// count valid thread only ( tids )
    fn get_threads(&self) -> Vec<usize>;
    /// deadlock or not
    fn detect(&self) -> bool {
        let threads = self.get_threads();
        let mut work = self.get_available();
        let need = self.get_need();
        let allocation = self.get_allocation();
        let mut finish = BTreeMap::new();

        // fill container
        threads.iter().for_each(|tid| {
            if need.contains_key(tid) {
                finish.insert(*tid, false);
            } else {
                finish.insert(*tid, true);
            }
        });

        // println!("[kernel] work: {:?}", work);
        // println!("[kernel] allocation: {:?}", allocation);
        // println!("[kernel] need: {:?}", need);

        loop {
            let tasks = finish
                .iter()
                .filter(|(tid, is_finish)| {
                    !*is_finish
                        && need[*tid]
                            .iter()
                            .enumerate()
                            .all(|(res_idx, res_num)| *res_num <= work[res_idx])
                })
                .map(|(idx, _)| *idx)
                .collect::<Vec<_>>();

            let cur_thread = tasks.get(0);

            if let Some(cur_thread_idx) = cur_thread {
                finish.insert(*cur_thread_idx, true);

                // release res in allocation if exsist

                allocation.get(cur_thread_idx).map(|vector| {
                    vector
                        .iter()
                        .enumerate()
                        .for_each(|(res_idx, res_nr)| work[res_idx] += res_nr)
                });

                break;
            } else {
                break;
            }
        }

        // println!("[kernel]: finish {:?}", finish);

        if finish.iter().all(|(_, is_finish)| *is_finish) {
            false
        } else {
            true
        }
    }
}
