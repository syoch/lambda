use std::fmt;

/// ラムダ式を表す列挙型（de Bruijnインデックス）
/// 変数は束縛位置までの距離で表現される
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeBruijn {
    /// 束縛変数: Var(index)
    /// 0 = 最も内側のλ, 1 = その外側のλ, ...
    Var(usize),
    /// 関数抽象: Abs(body)
    /// パラメータ名は不要
    Abs(Box<DeBruijn>),
    /// 関数適用: App(function, argument)
    App(Box<DeBruijn>, Box<DeBruijn>),
}

impl DeBruijn {
    /// β簡約（1ステップ）
    pub fn beta_reduce_step(&self) -> Option<Self> {
        match self {
            DeBruijn::App(m, n) => {
                if let DeBruijn::Abs(body) = m.as_ref() {
                    // β簡約を実行: (λ.M) N → M[0 := N]
                    Some(body.substitute(0, n))
                } else {
                    // 左辺を簡約
                    if let Some(m_reduced) = m.beta_reduce_step() {
                        Some(DeBruijn::App(Box::new(m_reduced), n.clone()))
                    } else {
                        // 右辺を簡約
                        n.beta_reduce_step()
                            .map(|n_reduced| DeBruijn::App(m.clone(), Box::new(n_reduced)))
                    }
                }
            }
            DeBruijn::Abs(body) => {
                // 本体を簡約
                body.beta_reduce_step()
                    .map(|body_reduced| DeBruijn::Abs(Box::new(body_reduced)))
            }
            DeBruijn::Var(_) => None,
        }
    }

    /// 変数の置換 M[idx := N]
    /// idxは置換する変数のインデックス、replacementは置換する項
    fn substitute(&self, idx: usize, replacement: &DeBruijn) -> Self {
        self.substitute_helper(idx, replacement, 0)
    }

    /// 置換のヘルパー関数
    /// depth: 現在のλの深さ（シフト量の調整に使用）
    fn substitute_helper(&self, idx: usize, replacement: &DeBruijn, depth: usize) -> Self {
        match self {
            DeBruijn::Var(n) => {
                if *n == idx + depth {
                    // 置換対象の変数：replacementをシフトして挿入
                    replacement.shift(depth as isize, 0)
                } else if *n > idx + depth {
                    // 外側の変数：インデックスを1減らす
                    DeBruijn::Var(n - 1)
                } else {
                    // それ以外：そのまま
                    DeBruijn::Var(*n)
                }
            }
            DeBruijn::Abs(body) => {
                // λの下に入るので深さを1増やす
                let new_body = body.substitute_helper(idx, replacement, depth + 1);
                DeBruijn::Abs(Box::new(new_body))
            }
            DeBruijn::App(m, n) => {
                let new_m = m.substitute_helper(idx, replacement, depth);
                let new_n = n.substitute_helper(idx, replacement, depth);
                DeBruijn::App(Box::new(new_m), Box::new(new_n))
            },
        }
    }

    /// インデックスのシフト
    /// shift > 0: インデックスを増やす（外側に持ち上げる）
    /// shift < 0: インデックスを減らす
    /// cutoff: この値以上のインデックスのみシフトする
    pub fn shift(&self, shift: isize, cutoff: usize) -> Self {
        self.shift_impl(shift, cutoff)
    }

    fn shift_impl(&self, shift: isize, cutoff: usize) -> Self {
        // シフト量が0の場合は何もしない
        if shift == 0 {
            return self.clone();
        }
        match self {
            DeBruijn::Var(n) => {
                if *n >= cutoff {
                    DeBruijn::Var((*n as isize + shift) as usize)
                } else {
                    DeBruijn::Var(*n)
                }
            }
            DeBruijn::Abs(body) => {
                // λの下ではcutoffを1増やす
                DeBruijn::Abs(Box::new(body.shift_impl(shift, cutoff + 1)))
            }
            DeBruijn::App(m, n) => {
                DeBruijn::App(
                    Box::new(m.shift_impl(shift, cutoff)),
                    Box::new(n.shift_impl(shift, cutoff))
                )
            },
        }
    }

    /// 正規形まで簡約（所有権を消費）
    /// max_node_count を超えた場合、早期終了する
    pub fn normalize(mut self, max_steps: usize) -> Self {
        let max_node_count = 100000; // ノード数の上限
        
        for step in 0..max_steps {
            // ノード数チェック（10ステップごと）
            if step % 10 == 0 && step > 0 {
                let node_count = self.node_count();
                if node_count > max_node_count {
                    eprintln!("    [normalize] step {}: node_count={} exceeded limit, early exit", step, node_count);
                    break;
                }
            }
            
            if let Some(next) = self.beta_reduce_step() {
                self = next;
                // 100ステップごとにログ出力（環境変数でデバッグモード）
                if step > 0 && step % 100 == 0 {
                    if std::env::var("DEBUG_NORMALIZE").is_ok() {
                        eprintln!("    [normalize] step {}/{}, node_count={}", step, max_steps, self.node_count());
                    }
                }
            } else {
                break;
            }
        }
        self
    }

    /// 正規形まで簡約（参照版・互換性のため）
    pub fn normalize_ref(&self, max_steps: usize) -> Self {
        self.clone().normalize(max_steps)
    }

    /// ノード数を計算
    pub fn node_count(&self) -> usize {
        match self {
            DeBruijn::Var(_) => 1,
            DeBruijn::Abs(body) => 1 + body.node_count(),
            DeBruijn::App(m, n) => 1 + m.node_count() + n.node_count(),
        }
    }
}

impl fmt::Display for DeBruijn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeBruijn::Var(n) => write!(f, "{}", n),
            DeBruijn::Abs(body) => {
                // 連続したλを見やすくする
                if let DeBruijn::Abs(_) = body.as_ref() {
                    write!(f, "λ.{}", body)
                } else {
                    write!(f, "λ.{}", body)
                }
            }
            DeBruijn::App(m, n) => {
                let m_str = match m.as_ref() {
                    DeBruijn::Abs(_) => format!("({})", m),
                    _ => format!("{}", m),
                };
                let n_str = match n.as_ref() {
                    DeBruijn::Var(_) => format!("{}", n),
                    _ => format!("({})", n),
                };
                write!(f, "{} {}", m_str, n_str)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === de Bruijnインデックステスト ===

    #[test]
    fn test_debruijn_identity() {
        // λ.0 (identity function)
        let id = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        assert_eq!(format!("{}", id), "λ.0");
    }

    #[test]
    fn test_debruijn_application() {
        // (λ.0) 0 - identity applied to a free variable
        let id = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        let app = DeBruijn::App(Rc::new(id), Rc::new(DeBruijn::Var(0)));
        assert_eq!(format!("{}", app), "(λ.0) 0");
    }

    #[test]
    fn test_debruijn_const() {
        // λ.λ.1 (K combinator)
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        assert_eq!(format!("{}", k), "λ.λ.1");
    }

    #[test]
    fn test_debruijn_beta_reduction() {
        // (λ.0) (λ.0) → λ.0
        let id = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        let app = DeBruijn::App(Rc::new(id.clone()), Rc::new(id.clone()));
        let reduced = app.beta_reduce_step().unwrap();
        assert_eq!(reduced, id);
    }

    #[test]
    fn test_debruijn_substitution() {
        // (λ.λ.1) 0 の簡約をテスト
        // K combinator: λ.λ.1 (これは λx.λy.x)
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        let a = DeBruijn::Var(0);

        // (λ.λ.1) 0 を1ステップ簡約
        let app = DeBruijn::App(Rc::new(k), Rc::new(a));
        let result = app.beta_reduce_step().unwrap();

        // 結果は λ.1 (aがシフトされて1になる)
        assert_eq!(result, DeBruijn::Abs(Rc::new(DeBruijn::Var(1))));
    }

    #[test]
    fn test_debruijn_normalize() {
        // (λ.λ.1) 0 1 → 0
        // K combinator に2つの引数を適用
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        let a = DeBruijn::Var(0);
        let b = DeBruijn::Var(1);

        let expr = DeBruijn::App(Rc::new(DeBruijn::App(Rc::new(k), Rc::new(a))), Rc::new(b));

        let normalized = expr.normalize(10);
        assert_eq!(normalized, DeBruijn::Var(0));
    }

    #[test]
    fn test_debruijn_s_combinator() {
        // S combinator: λ.λ.λ.2 0 (1 0)
        let s = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(
            DeBruijn::App(
                Rc::new(DeBruijn::App(
                    Rc::new(DeBruijn::Var(2)),
                    Rc::new(DeBruijn::Var(0)),
                )),
                Rc::new(DeBruijn::App(
                    Rc::new(DeBruijn::Var(1)),
                    Rc::new(DeBruijn::Var(0)),
                )),
            ),
        ))))));

        // Displayの実装は括弧を最小限にする
        assert_eq!(format!("{}", s), "λ.λ.λ.2 0 (1 0)");
    }

    #[test]
    fn test_debruijn_node_count() {
        // λ.0 のノード数は 2 (Abs + Var)
        let id = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        assert_eq!(id.node_count(), 2);

        // λ.λ.1 のノード数は 3 (Abs + Abs + Var)
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        assert_eq!(k.node_count(), 3);

        // (λ.0) 0 のノード数は 4 (App + Abs + Var + Var)
        let app = DeBruijn::App(Rc::new(id), Rc::new(DeBruijn::Var(0)));
        assert_eq!(app.node_count(), 4);
    }

    #[test]
    fn test_debruijn_complex_reduction() {
        // ((λ.λ.1) (λ.0)) (λ.0) → λ.0
        // K I I → I
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        let i = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));

        let expr = DeBruijn::App(
            Rc::new(DeBruijn::App(Rc::new(k), Rc::new(i.clone()))),
            Rc::new(i.clone()),
        );

        let normalized = expr.normalize(10);
        assert_eq!(normalized, i);
    }

    #[test]
    fn test_debruijn_shift() {
        // λ.0 を外側にシフト
        let id = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        let shifted = id.shift(1, 0);
        // cutoff=0、Absで深さが1増えるので、内部ではcutoff=1になる
        // Var(0)は0 < 1なのでシフトされない
        assert_eq!(shifted, DeBruijn::Abs(Rc::new(DeBruijn::Var(0))));
    }

    #[test]
    fn test_debruijn_equality() {
        // 同じ構造は等しい
        let id1 = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        let id2 = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        assert_eq!(id1, id2);

        // 異なる構造は等しくない
        let k = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))));
        assert_ne!(id1, k);
    }

    #[test]
    fn test_debruijn_hash() {
        use std::collections::HashSet;

        // HashSetで重複を検出できる
        let mut set = HashSet::new();
        let id1 = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        let id2 = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));

        set.insert(id1.clone());
        set.insert(id2);

        // 同じ構造なので1つだけ
        assert_eq!(set.len(), 1);
    }
}
