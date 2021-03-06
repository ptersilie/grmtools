// Copyright (c) 2018 King's College London
// created by the Software Development Team <http://soft-dev.org/>
//
// The Universal Permissive License (UPL), Version 1.0
//
// Subject to the condition set forth below, permission is hereby granted to any person obtaining a
// copy of this software, associated documentation and/or data (collectively the "Software"), free
// of charge and under any and all copyright rights in the Software, and any and all patent rights
// owned or freely licensable by each licensor hereunder covering either (i) the unmodified
// Software as contributed to or provided by such licensor, or (ii) the Larger Works (as defined
// below), to deal in both
//
// (a) the Software, and
// (b) any piece of software and/or hardware listed in the lrgrwrks.txt file
// if one is included with the Software (each a "Larger Work" to which the Software is contributed
// by such licensors),
//
// without restriction, including without limitation the rights to copy, create derivative works
// of, display, perform, and distribute the Software and make, use, sell, offer for sale, import,
// export, have made, and have sold the Software and the Larger Work(s), and to sublicense the
// foregoing rights on either these or other terms.
//
// This license is subject to the following condition: The above copyright notice and either this
// complete permission notice or at a minimum a reference to the UPL must be included in all copies
// or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::marker::PhantomData;

use num_traits::{AsPrimitive, PrimInt, Unsigned};
use vob::Vob;

use yacc::YaccGrammar;
use RIdx;
use Symbol;
use TIdx;

/// `Follows` stores all the Follow sets for a given grammar. For example, given this code and
/// grammar:
/// ```ignore
///   let grm = YaccGrammar::new(YaccKind::Original, "
///       S: A 'b';
///       A: 'a' | ;
///     ").unwrap();
///   let follows = Follows::new(&grm);
/// ```
/// then the following assertions (and only the following assertions) about the Follows set are
/// correct:
/// ```ignore
///   assert!(follows.is_set(grm.rule_idx("S").unwrap(), grm.eof_token_idx());
///   assert!(follows.is_set(grm.rule_idx("A").unwrap(), grm.token_idx("b").unwrap()));
/// ```
#[derive(Debug)]
pub struct YaccFollows<StorageT> {
    follows: Vec<Vob>,
    phantom: PhantomData<StorageT>
}

impl<StorageT: 'static + PrimInt + Unsigned> YaccFollows<StorageT>
where
    usize: AsPrimitive<StorageT>
{
    /// Generates and returns the Follows set for the given grammar.
    pub fn new(grm: &YaccGrammar<StorageT>) -> Self {
        let mut follows = Vec::with_capacity(usize::from(grm.rules_len()));
        for _ in grm.iter_rules() {
            follows.push(Vob::from_elem(usize::from(grm.tokens_len()), false));
        }
        follows[usize::from(grm.start_rule_idx())].set(usize::from(grm.eof_token_idx()), true);

        let firsts = grm.firsts();
        loop {
            let mut changed = false;
            for pidx in grm.iter_pidxs() {
                let ridx = grm.prod_to_rule(pidx);
                let prod = grm.prod(pidx);
                // Our implementation of the Follows algorithm is simple: we start from the right
                // hand side of a production and work backwards. While epsilon is true, any
                // nonterminals we encounter have the Follow set of the production's rule added to
                // them. As soon as we hit a token or a nonterminal that can't produce the empty
                // string, we set epsilon to false. At that point, we simply add the first set of
                // the following symbol to any nonterminals we encounter.
                let mut epsilon = true;
                for sidx in (0..prod.len()).rev() {
                    let sym = prod[sidx];
                    match sym {
                        Symbol::Token(_) => {
                            epsilon = false;
                        }
                        Symbol::Rule(s_ridx) => {
                            if epsilon {
                                for tidx in grm.iter_tidxs() {
                                    if follows[usize::from(ridx)][usize::from(tidx)]
                                        && follows[usize::from(s_ridx)].set(usize::from(tidx), true)
                                    {
                                        changed = true;
                                    }
                                }
                            }
                            if !firsts.is_epsilon_set(s_ridx) {
                                epsilon = false;
                            }
                            if sidx < prod.len() - 1 {
                                match prod[sidx + 1] {
                                    Symbol::Token(nxt_tidx) => {
                                        if follows[usize::from(s_ridx)]
                                            .set(usize::from(nxt_tidx), true)
                                        {
                                            changed = true;
                                        }
                                    }
                                    Symbol::Rule(nxt_ridx) => {
                                        if follows[usize::from(s_ridx)].or(firsts.firsts(nxt_ridx))
                                        {
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !changed {
                return YaccFollows {
                    follows,
                    phantom: PhantomData
                };
            }
        }
    }

    /// Return the Follows `Vob` for rule `ridx`.
    pub fn follows(&self, ridx: RIdx<StorageT>) -> &Vob {
        &self.follows[usize::from(ridx)]
    }

    /// Returns true if the token `tidx` is in the follow set for rule `ridx`.
    pub fn is_set(&self, ridx: RIdx<StorageT>, tidx: TIdx<StorageT>) -> bool {
        self.follows[usize::from(ridx)][usize::from(tidx)]
    }
}

#[cfg(test)]
mod test {
    use super::YaccFollows;
    use num_traits::{AsPrimitive, PrimInt, Unsigned};
    use yacc::{YaccGrammar, YaccKind};

    fn has<StorageT: 'static + PrimInt + Unsigned>(
        grm: &YaccGrammar<StorageT>,
        follows: &YaccFollows<StorageT>,
        rn: &str,
        should_be: Vec<&str>
    ) where
        usize: AsPrimitive<StorageT>
    {
        let ridx = grm.rule_idx(rn).unwrap();
        for tidx in grm.iter_tidxs() {
            let n = if tidx == grm.eof_token_idx() {
                "$"
            } else {
                grm.token_name(tidx).unwrap_or("<no name>")
            };
            if should_be.iter().find(|&x| x == &n).is_none() {
                if follows.is_set(ridx, tidx) {
                    panic!("{} is incorrectly set in {}", n, rn);
                }
            } else {
                if !follows.is_set(ridx, tidx) {
                    panic!("{} is not set in {}", n, rn);
                }
            }
        }
    }

    #[test]
    fn test_follow() {
        // Adapted from p2 of https://www.cs.uaf.edu/~cs331/notes/FirstFollow.pdf
        let grm = YaccGrammar::new(
            YaccKind::Original,
            &"
                %start E
                %%
                E: T E2 ;
                E2: '+' T E2 | ;
                T: F T2 ;
                T2: '*' F T2 | ;
                F: '(' E ')' | 'ID' ;
          "
        )
        .unwrap();
        let follows = grm.follows();
        has(&grm, &follows, "E", vec![")", "$"]);
        has(&grm, &follows, "E2", vec![")", "$"]);
        has(&grm, &follows, "T", vec!["+", ")", "$"]);
        has(&grm, &follows, "T2", vec!["+", ")", "$"]);
        has(&grm, &follows, "F", vec!["+", "*", ")", "$"]);
    }

    #[test]
    fn test_follow2() {
        // Adapted from https://www.l2f.inesc-id.pt/~david/w/pt/Top-Down_Parsing/Exercise_5:_Test_2010/07/01
        let grm = YaccGrammar::new(
            YaccKind::Original,
            &"
                %start A
                %%
                A : 't' B2 D | 'v' D2 ;
                B : 't' B2 | ;
                B2: 'w' B | 'u' 'w' B ;
                D : 'v' D2 ;
                D2: 'x' B D2 | ;
          "
        )
        .unwrap();
        let follows = grm.follows();
        has(&grm, &follows, "A", vec!["$"]);
        has(&grm, &follows, "B", vec!["v", "x", "$"]);
        has(&grm, &follows, "B2", vec!["v", "x", "$"]);
        has(&grm, &follows, "D", vec!["$"]);
        has(&grm, &follows, "D2", vec!["$"]);
    }

    #[test]
    fn test_follow3() {
        let grm = YaccGrammar::new(
            YaccKind::Original,
            &"
                %start S
                %%
                S: A 'b';
                A: 'b' | ;
          "
        )
        .unwrap();
        let follows = grm.follows();
        has(&grm, &follows, "S", vec!["$"]);
        has(&grm, &follows, "A", vec!["b"]);
    }

    #[test]
    fn test_follow_corchuelo() {
        let grm = YaccGrammar::new(
            YaccKind::Original,
            &"
                %start E
                %%
                E : 'N'
                  | E '+' 'N'
                  | '(' E ')'
                  ;
          "
        )
        .unwrap();
        let follows = grm.follows();
        has(&grm, &follows, "E", vec!["+", ")", "$"]);
    }
}
