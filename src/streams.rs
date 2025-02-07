use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::rc::Rc;

use num::bigint::{BigInt, Sign};
use num::ToPrimitive;

use crate::core::*;

#[derive(Debug, Clone)]
pub struct Repeat(pub Obj);
impl Iterator for Repeat {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        Some(Ok(self.0.clone()))
    }
}
impl Display for Repeat {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "repeat({})", self.0)
    }
}
impl Stream for Repeat {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        None
    }
    fn force(&self) -> NRes<Vec<Obj>> {
        Err(NErr::value_error(
            "Cannot force repeat because it's infinite".to_string(),
        ))
    }
    fn pythonic_index_isize(&self, _: isize) -> NRes<Obj> {
        Ok(self.0.clone())
    }
    fn pythonic_slice(&self, lo: Option<isize>, hi: Option<isize>) -> NRes<Seq> {
        let lo = match lo {
            Some(x) => {
                if x < 0 {
                    x - 1
                } else {
                    x
                }
            }
            None => 0,
        };
        let hi = match hi {
            Some(x) => {
                if x < 0 {
                    x - 1
                } else {
                    x
                }
            }
            None => -1,
        };
        Ok(match (lo < 0, hi < 0) {
            (true, true) | (false, false) => {
                Seq::List(Rc::new(vec![self.0.clone(); (hi - lo).max(0) as usize]))
            }
            (true, false) => Seq::List(Rc::new(Vec::new())),
            (false, true) => Seq::Stream(Rc::new(self.clone())),
        })
    }
    fn reversed(&self) -> NRes<Seq> {
        Ok(Seq::Stream(Rc::new(self.clone())))
    }
}
#[derive(Debug, Clone)]
// just gonna say this has to be nonempty
pub struct Cycle(pub Rc<Vec<Obj>>, pub usize);
impl Iterator for Cycle {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let ret = self.0[self.1].clone();
        self.1 = (self.1 + 1) % self.0.len();
        Some(Ok(ret))
    }
}
impl Display for Cycle {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "cycle({})", CommaSeparated(&**self.0))
    }
}
impl Stream for Cycle {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        None
    }
    fn force(&self) -> NRes<Vec<Obj>> {
        Err(NErr::value_error(
            "Cannot force cycle because it's infinite".to_string(),
        ))
    }
    fn pythonic_index_isize(&self, i: isize) -> NRes<Obj> {
        Ok(self.0[(self.1 as isize + i).rem_euclid(self.0.len() as isize) as usize].clone())
    }
    fn reversed(&self) -> NRes<Seq> {
        let mut v: Vec<Obj> = (*self.0).clone();
        v.reverse();
        // 0 -> 0, 1 -> n - 1...
        let i = (v.len() - self.1) % v.len();
        Ok(Seq::Stream(Rc::new(Cycle(Rc::new(v), i))))
    }
}
#[derive(Debug, Clone)]
pub struct Range(pub BigInt, pub Option<BigInt>, pub BigInt);
impl Range {
    fn empty(&self) -> bool {
        let Range(start, end, step) = self;
        match (step.sign(), end) {
            (_, None) => false,
            (Sign::Minus, Some(end)) => start <= end,
            (Sign::NoSign | Sign::Plus, Some(end)) => start >= end,
        }
    }
}
impl Iterator for Range {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        if self.empty() {
            None
        } else {
            let Range(start, _end, step) = self;
            let ret = start.clone();
            *start += &*step;
            Some(Ok(Obj::from(ret)))
        }
    }
}
impl Display for Range {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            Some(end) => write!(formatter, "{} til {} by {}", self.0, end, self.2),
            None => write!(formatter, "{} til ... by {}", self.0, self.2),
        }
    }
}
impl Stream for Range {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        let Range(start, end, step) = self;
        let end = end.as_ref()?;
        match step.sign() {
            // weird
            Sign::NoSign => {
                if start < end {
                    None
                } else {
                    Some(0)
                }
            }
            Sign::Minus => {
                ((end - start - step + 1usize).max(BigInt::from(0)) / (-step)).to_usize()
            }
            Sign::Plus => ((end - start + step - 1usize).max(BigInt::from(0)) / step).to_usize(),
        }
    }
}

// Order: lexicographic indexes
#[derive(Debug, Clone)]
pub struct Permutations(pub Rc<Vec<Obj>>, pub Option<Rc<Vec<usize>>>);
impl Iterator for Permutations {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let v = Rc::make_mut(self.1.as_mut()?);
        let ret = Obj::list(v.iter().map(|i| self.0[*i].clone()).collect());

        // 1 6 4 2 -> 2 1 4 6
        // last increase, and the largest index of something larger than it
        let mut up = None;
        for i in 0..(v.len() - 1) {
            if v[i] < v[i + 1] {
                up = Some((i, i + 1));
            } else {
                match &mut up {
                    Some((inc, linc)) => {
                        if v[i + 1] > v[*inc] {
                            *linc = i + 1;
                        }
                    }
                    None => {}
                }
            }
        }
        match up {
            Some((inc, linc)) => {
                v.swap(inc, linc);
                v[inc + 1..].reverse();
            }
            None => {
                self.1 = None;
            }
        }
        Some(Ok(ret))
    }
}
impl Display for Permutations {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            Some(x) => {
                write!(
                    formatter,
                    "permutations({} @ {})",
                    CommaSeparated(&**self.0),
                    CommaSeparated(&**x)
                )
            }
            None => write!(formatter, "permutations(done)"),
        }
    }
}
impl Stream for Permutations {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        match &self.1 {
            None => Some(0),
            Some(v) => {
                let mut cur = 1usize;
                Some(
                    (1..v.len())
                        .map(|i| {
                            // Each way we could replace v[len - 1 - i] with a later number that's larger
                            // gives us cur.
                            // i = 0, cur = undef
                            // i = 1, cur = 1
                            // i = 2, cur = 2
                            // i = 3, cur = 6
                            cur *= i;
                            cur * (v.len() - i..v.len())
                                .filter(|j| v[*j] > v[v.len() - 1 - i])
                                .count()
                        })
                        .sum::<usize>()
                        + 1usize,
                )
            }
        }
    }
}

// Order: lexicographic indexes
#[derive(Debug, Clone)]
pub struct Combinations(pub Rc<Vec<Obj>>, pub Option<Rc<Vec<usize>>>);
impl Iterator for Combinations {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let v = Rc::make_mut(self.1.as_mut()?);
        if v.len() > self.0.len() {
            return None;
        }
        let ret = Obj::list(v.iter().map(|i| self.0[*i].clone()).collect());

        let mut last = self.0.len();
        for i in (0..v.len()).rev() {
            if v[i] + 1 < last {
                // found the next
                v[i] += 1;
                for j in i + 1..v.len() {
                    v[j] = v[j - 1] + 1;
                }
                return Some(Ok(ret));
            }
            last -= 1;
        }
        self.1 = None;
        Some(Ok(ret))
    }
}
impl Display for Combinations {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            Some(x) => {
                write!(
                    formatter,
                    "combinations({} @ {})",
                    CommaSeparated(&**self.0),
                    CommaSeparated(&**x)
                )
            }
            None => write!(formatter, "combinations(done)"),
        }
    }
}
impl Stream for Combinations {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    // FIXME this math is hard
    /*
    fn len(&self) -> Option<usize> {
        match &self.1 {
            None => Some(0),
            Some(v) => {
                Some((0..v.len()).rev().map(|i| {
                    // ...
                }).sum::<usize>() + 1usize)
            }
        }
    }
    */
}

// Order: big-endian binary
#[derive(Debug, Clone)]
pub struct Subsequences(pub Rc<Vec<Obj>>, pub Option<Rc<Vec<bool>>>);
impl Iterator for Subsequences {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let v = Rc::make_mut(self.1.as_mut()?);
        let ret = Obj::list(
            v.iter()
                .zip(self.0.iter())
                .filter_map(|(b, x)| if *b { Some(x.clone()) } else { None })
                .collect(),
        );

        for i in (0..v.len()).rev() {
            if !v[i] {
                v[i] = true;
                for j in i + 1..v.len() {
                    v[j] = false;
                }
                return Some(Ok(ret));
            }
        }
        self.1 = None;
        Some(Ok(ret))
    }
}
impl Display for Subsequences {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            Some(x) => {
                write!(
                    formatter,
                    "subsequences({} @ {})",
                    CommaSeparated(&**self.0),
                    CommaSeparated(&**x)
                )
            }
            None => write!(formatter, "subsequences(done)"),
        }
    }
}
impl Stream for Subsequences {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        match &self.1 {
            None => Some(0),
            Some(v) => {
                let mut cur = 1usize;
                Some(
                    (0..v.len())
                        .rev()
                        .map(|i| {
                            let s = if !v[i] {
                                // If we keep everything before this and set this to true:
                                cur
                            } else {
                                0
                            };
                            cur *= 2;
                            s
                        })
                        .sum::<usize>()
                        + 1usize,
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CartesianPower(pub Rc<Vec<Obj>>, pub Option<Rc<Vec<usize>>>);
impl Iterator for CartesianPower {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let v = Rc::make_mut(self.1.as_mut()?);
        let ret = Obj::list(v.iter().map(|i| self.0[*i].clone()).collect());

        // let mut last = self.0.len();
        for i in (0..v.len()).rev() {
            v[i] += 1;
            if v[i] == self.0.len() {
                v[i] = 0;
            } else {
                return Some(Ok(ret));
            }
        }
        self.1 = None;
        Some(Ok(ret))
    }
}
impl Display for CartesianPower {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            Some(x) => {
                write!(
                    formatter,
                    "CartesianPower({} @ {})",
                    CommaSeparated(&**self.0),
                    CommaSeparated(&**x)
                )
            }
            None => write!(formatter, "CartesianPower(done)"),
        }
    }
}
impl Stream for CartesianPower {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        match &self.1 {
            None => Some(0),
            Some(v) => {
                let mut cur = 1usize;
                Some(
                    (0..v.len())
                        .rev()
                        .map(|i| {
                            // If we keep everything before this and increase this:
                            let s = (self.0.len() - 1 - v[i]) * cur;
                            cur *= self.0.len();
                            s
                        })
                        .sum::<usize>()
                        + 1usize,
                )
            }
        }
    }
}

// moderately illegal
// we'll treat NErr::Break as graceful termination
#[derive(Clone)]
pub struct Iterate(pub NRes<(Obj, Func, REnv)>);
// directly debug-printing env can easily recurse infinitely
impl Debug for Iterate {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.0 {
            Ok((obj, func, _)) => write!(fmt, "Iterate({:?}, {:?}, ...)", obj, func),
            Err(NErr::Break(None)) => write!(fmt, "Iterate(stopped)"),
            Err(e) => write!(fmt, "Iterate(ERROR: {:?})", e),
        }
    }
}
impl Iterator for Iterate {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        match &mut self.0 {
            Ok((obj, func, renv)) => {
                let ret = obj.clone();
                let cur = std::mem::take(obj);
                match func.run(&renv, vec![cur]) {
                    Ok(nxt) => {
                        *obj = nxt;
                    }
                    Err(e) => {
                        self.0 = Err(e);
                    }
                }
                Some(Ok(ret))
            }
            Err(NErr::Break(None)) => None,
            Err(e) => Some(Err(e.clone())),
        }
    }
}
impl Display for Iterate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Ok((obj, func, _)) => write!(formatter, "Iterate({}, {}, ...)", obj, func),
            Err(NErr::Break(None)) => write!(formatter, "Iterate(stopped)"),
            Err(e) => write!(formatter, "Iterate(ERROR: {})", e),
        }
    }
}
impl Stream for Iterate {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    fn len(&self) -> Option<usize> {
        None
    }
}

// maybe even more illegal? not sure
// again we'll treat NErr::Break as graceful termination
pub struct MappedStream(pub NRes<(Box<dyn Stream>, Func, REnv)>);
impl Clone for MappedStream {
    fn clone(&self) -> MappedStream {
        match &self.0 {
            Err(e) => MappedStream(Err(e.clone())),
            Ok((inner, func, renv)) => {
                MappedStream(Ok((inner.clone_box(), func.clone(), renv.clone())))
            }
        }
    }
}
// directly debug-printing env can easily recurse infinitely
impl Debug for MappedStream {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.0 {
            Err(NErr::Break(None)) => write!(fmt, "MappedStream(stopped)"),
            Err(e) => write!(fmt, "MappedStream(ERROR: {:?})", e),
            Ok((inner, func, _)) => write!(fmt, "MappedStream({:?}, {:?}, ...)", inner, func),
        }
    }
}
impl Iterator for MappedStream {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let (inner, func, renv) = self.0.as_mut().ok()?;
        match inner.next() {
            Some(Err(e)) => {
                self.0 = Err(e.clone());
                Some(Err(e))
            }
            Some(Ok(cur)) => match func.run(&renv, vec![cur]) {
                Ok(nxt) => Some(Ok(nxt)),
                Err(e) => {
                    self.0 = Err(e.clone());
                    Some(Err(e))
                }
            },
            None => {
                self.0 = Err(NErr::Break(None));
                None
            }
        }
    }
}
impl Display for MappedStream {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Ok((inner, func, _)) => write!(formatter, "MappedStream({}, {}, ...)", inner, func),
            Err(e) => write!(formatter, "MappedStream(ERROR: {})", e),
        }
    }
}
impl Stream for MappedStream {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    /*
    fn len(&self) -> Option<usize> {
        match &self.0 {
            Ok((inner, _, _)) => inner.len(),
            Err(_) => Some(0),
        }
    }
    */
}
pub struct StridedStream(pub NRes<(Box<dyn Stream>, usize, usize)>);
impl Clone for StridedStream {
    fn clone(&self) -> StridedStream {
        match &self.0 {
            Err(e) => StridedStream(Err(e.clone())),
            Ok((inner, stride, pos)) => {
                StridedStream(Ok((inner.clone_box(), stride.clone(), pos.clone())))
            }
        }
    }
}
// directly debug-printing env can easily recurse infinitely
impl Debug for StridedStream {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.0 {
            Err(NErr::Break(None)) => write!(fmt, "StridedStream(stopped)"),
            Err(e) => write!(fmt, "StridedStream(ERROR: {:?})", e),
            Ok((inner, stride, size)) => write!(fmt, "StridedStream({:?}, {:?}, {:?},...)", inner, stride, size),
        }
    }
}
impl Iterator for StridedStream {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let (inner, stride, size) = self.0.as_mut().ok()?;
        loop {
            match inner.next() {
                Some(Err(e)) => {
                    self.0 = Err(e.clone());
                    return Some(Err(e))
                }
                Some(Ok(cur)) => {
                    if *size % *stride == 0 {
                        *size += 1;
                        return Some(Ok(cur));
                    }
                    *size += 1;
                },
                None => {
                    self.0 = Err(NErr::Break(None));
                    return None;
                }
            }
        }
    }
}
impl Display for StridedStream {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Ok((inner, stride, pos)) => write!(formatter, "StridedStream({}, {}, {}, ...)", inner, stride, pos),
            Err(e) => write!(formatter, "StridedStream(ERROR: {})", e),
        }
    }
}
impl Stream for StridedStream {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    /*
    fn len(&self) -> Option<usize> {
        match &self.0 {
            Ok((inner, _, _)) => inner.len(),
            Err(_) => Some(0),
        }
    }
    */
}

// TODO: remove ScannedStream and MappedStream with dyn Iterator i.e. type erased iterators
pub struct ScannedStream(pub NRes<(Box<dyn Stream>, Obj, Func, REnv)>, pub Option<Obj>);
impl Clone for ScannedStream {
    fn clone(&self) -> ScannedStream {
        match &self.0 {
            Err(e) => ScannedStream(Err(e.clone()), None),
            Ok((inner, init, func, renv)) => {
                ScannedStream(Ok((inner.clone_box(), init.clone(), func.clone(), renv.clone())), self.1.clone())
            }
        }
    }
}
// directly debug-printing env can easily recurse infinitely
impl Debug for ScannedStream {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.0 {
            Err(NErr::Break(None)) => write!(fmt, "ScannedStream(stopped)"),
            Err(e) => write!(fmt, "ScannedStream(ERROR: {:?})", e),
            Ok((inner, init, func, _)) => write!(fmt, "ScannedStream({:?}, {:?}, {:?}, ...)", inner, init, func),
        }
    }
}
impl Iterator for ScannedStream {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let (inner, init, func, renv) = self.0.as_mut().ok()?;
        
        if let Some(acc) = self.1.take() {
            match inner.next() {
                Some(Err(e)) => {
                    self.0 = Err(e.clone());
                    Some(Err(e))
                }
                Some(Ok(cur)) => match func.run(&renv, vec![acc, cur]) {
                    Ok(nxt) => {
                        self.1 = Some(nxt.clone());
                        Some(Ok(nxt))
                    },
                    Err(e) => {
                        self.0 = Err(e.clone());
                        Some(Err(e))
                    }
                },
                None => {
                    self.0 = Err(NErr::Break(None));
                    None
                }
            }
        } else {
            self.1 = Some(init.clone());
            Some(Ok(init.clone()))
        }
    }
}
impl Display for ScannedStream {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Ok((inner, init, func, _)) => write!(formatter, "ScannedStream({}, {}, {}, ...)", inner, init, func),
            Err(e) => write!(formatter, "ScannedStream(ERROR: {})", e),
        }
    }
}
impl Stream for ScannedStream {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
    /*
    fn len(&self) -> Option<usize> {
        match &self.0 {
            Ok((inner, _, _)) => inner.len(),
            Err(_) => Some(0),
        }
    }
    */
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct TotalOrderWrapper(Obj);

impl Eq for TotalOrderWrapper {}

impl Ord for TotalOrderWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.partial_cmp(other) {
            Some(o) => o,
            None => std::cmp::Ordering::Equal
        }
        // match (self.0, other.0) {
        //     (Obj::Num(a), Obj::Num(b)) => a.cmp(b),
        //     (Obj::Seq(a), Obj::Seq(b)) => a.cmp(b),
        //     _ => std::cmp::Ordering::Equal,
        // }
    }
}

#[derive(Debug, Clone)]
pub struct HeapStream(NRes<(std::collections::BinaryHeap<TotalOrderWrapper>, Func, REnv)>);

impl HeapStream {
    pub fn new(o: Obj, f: Func, renv : REnv) -> HeapStream {
        let mut heap = std::collections::BinaryHeap::<TotalOrderWrapper>::new();
        heap.push(TotalOrderWrapper(o));
        HeapStream(Ok((heap, f, renv)))
    }
}

impl Iterator for HeapStream {
    type Item = NRes<Obj>;
    fn next(&mut self) -> Option<NRes<Obj>> {
        let (heap, func, renv) = self.0.as_mut().ok()?;
        // This does not match LazyStream. Should error out Stream state for future next calls???
        let ret = func.run(renv, vec![heap.pop()?.0]).ok()?;

        if let Obj::Seq(Seq::List(v)) = ret.clone() {
             heap.extend(v.iter().map(|o| TotalOrderWrapper(o.clone())))
        } else {
            return Some(Err(NErr::type_error(format!("HeapStream func must return lists. Got {:?}", ret))));
        }
        Some(Ok(ret))
    }
}
impl Display for HeapStream {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "HeapStream(...)")
    }
}
impl Stream for HeapStream {
    fn clone_box(&self) -> Box<dyn Stream> {
        Box::new(self.clone())
    }
}