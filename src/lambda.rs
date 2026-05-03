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
    /// 正規形メモ: Memo(expr)
    /// [expr] で表現される。β簡約の引数にはなれない
    Memo(Box<DeBruijn>),
}

impl DeBruijn {
    /// インデックスnから変数名を生成 (1文字の場合: a-z, A-Z)
    fn var_name_single(n: usize) -> Option<String> {
        if n < 26 {
            Some((b'a' + n as u8) as char).map(|c| c.to_string())
        } else if n < 52 {
            Some((b'A' + (n - 26) as u8) as char).map(|c| c.to_string())
        } else {
            None
        }
    }

    /// De Bruijnインデックスをスコープ内の変数名に変換
    /// depth: 現在のAbs深度, var_idx: De Bruijnインデックス
    fn debruijn_to_var_name(depth: usize, var_idx: usize) -> String {
        let scope_var = depth.saturating_sub(1 + var_idx);
        Self::var_name_single(scope_var).unwrap_or_else(|| format!("x{}", scope_var))
    }

    /// Pretty print implementation
    pub fn pretty_print(&self, depth: usize) -> String {
        match self {
            DeBruijn::Var(n) => Self::debruijn_to_var_name(depth, *n),
            DeBruijn::Abs(body) => {
                let var_name = Self::debruijn_to_var_name(depth + 1, 0);
                format!("\\{}. {}", var_name, body.pretty_print(depth + 1))
            }
            DeBruijn::App(m, n) => {
                let m_str = match m.as_ref() {
                    DeBruijn::Abs(_) => {
                        format!("({})", m.pretty_print(depth))
                    }
                    _ => m.pretty_print(depth),
                };
                let n_str = match n.as_ref() {
                    DeBruijn::Var(_) => n.pretty_print(depth),
                    DeBruijn::Abs(_) => format!("({})", n.pretty_print(depth)),
                    DeBruijn::Memo(_) => n.pretty_print(depth),
                    _ => format!("({})", n.pretty_print(depth)),
                };
                format!("{} {}", m_str, n_str)
            }
            DeBruijn::Memo(expr) => {
                format!("[{}]", expr.pretty_print(depth))
            }
        }
    }

    /// 変数が自由に現れているかチェック
    /// depth: 現在のλの深さ（結合変数かどうかの判定に使用）
    fn has_free_var(&self, var: usize) -> bool {
        self.has_free_var_helper(var, 0)
    }

    fn has_free_var_helper(&self, var: usize, depth: usize) -> bool {
        match self {
            DeBruijn::Var(n) => {
                // 変数 n が var に一致するか
                // depthぶん潜っているため、探している変数のインデックスは var + depth になる
                *n == var + depth
            }
            DeBruijn::Abs(body) => {
                // λの下では深さを1増やす
                body.has_free_var_helper(var, depth + 1)
            }
            DeBruijn::App(m, n) => {
                m.has_free_var_helper(var, depth) || n.has_free_var_helper(var, depth)
            }
            DeBruijn::Memo(expr) => expr.has_free_var_helper(var, depth),
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
                    // depthぶん外側に持ち上げる必要がある
                    replacement.shift(depth as isize, 0)
                } else if *n > idx + depth {
                    // 置換対象より外側の変数：インデックスを1減らす
                    // （置換により1つのλが消えるため）
                    DeBruijn::Var(n - 1)
                } else {
                    // 置換対象より内側の変数：そのまま
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
            }
            DeBruijn::Memo(expr) => {
                let new_expr = expr.substitute_helper(idx, replacement, depth);
                DeBruijn::Memo(Box::new(new_expr))
            }
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
            DeBruijn::App(m, n) => DeBruijn::App(
                Box::new(m.shift_impl(shift, cutoff)),
                Box::new(n.shift_impl(shift, cutoff)),
            ),
            DeBruijn::Memo(expr) => DeBruijn::Memo(Box::new(expr.shift_impl(shift, cutoff))),
        }
    }

    pub fn normalize_step(&self) -> Option<Self> {
        match self {
            DeBruijn::App(m, n) => {
                if let DeBruijn::Abs(body) = m.as_ref() {
                    if let DeBruijn::Memo(_) = n.as_ref() {
                    } else {
                        return Some(body.substitute(0, n));
                    }
                }

                if let Some(m_reduced) = m.normalize_step() {
                    return Some(DeBruijn::App(Box::new(m_reduced), n.clone()));
                }

                if let Some(n_reduced) = n.normalize_step() {
                    return Some(DeBruijn::App(m.clone(), Box::new(n_reduced)));
                }
            }
            DeBruijn::Memo(expr) => {
                if let Some(next) = expr.normalize_step() {
                    return Some(DeBruijn::Memo(Box::new(next)));
                }
            }
            DeBruijn::Abs(body) => {
                if let DeBruijn::App(m, n) = body.as_ref() {
                    if let DeBruijn::Var(0) = n.as_ref() {
                        // λ.(M 0) の形式
                        // Mに0が自由に現れていないか確認
                        if !m.has_free_var(0) {
                            return Some(m.shift(-1, 0));
                        }
                    }
                }
                if let Some(next) = body.normalize_step() {
                    return Some(DeBruijn::Abs(Box::new(next)));
                }
            }
            _ => {}
        }

        None
    }

    /// 正規形まで簡約（所有権を消費）
    pub fn normalize(mut self, max_steps: usize) -> Self {
        for _ in 0..max_steps {
            if let Some(next) = self.normalize_step() {
                self = next;
            } else {
                break;
            }
        }

        self
    }

    /// ノード数を計算
    pub fn node_count(&self) -> usize {
        match self {
            DeBruijn::Var(_) => 1,
            DeBruijn::Abs(body) => 1 + body.node_count(),
            DeBruijn::App(m, n) => 1 + m.node_count() + n.node_count(),
            DeBruijn::Memo(expr) => 1 + expr.node_count(),
        }
    }
}

impl fmt::Display for DeBruijn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === de Bruijnインデックステスト ===

    #[test]
    fn test_debruijn_identity() {
        // λ.0 (identity function)
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        assert_eq!(format!("{}", id), "\\a. a");
    }

    #[test]
    fn test_debruijn_application() {
        // (λ.0) 0 - identity applied to a free variable
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let app = DeBruijn::App(Box::new(id), Box::new(DeBruijn::Var(0)));
        assert_eq!(format!("{}", app), "(\\a. a) a");
    }

    #[test]
    fn test_debruijn_const() {
        // λ.λ.1 (K combinator)
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        assert_eq!(format!("{}", k), "\\a. \\b. a");
    }

    #[test]
    fn test_debruijn_beta_reduction() {
        // (λ.0) (λ.0) → λ.0
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let app = DeBruijn::App(Box::new(id.clone()), Box::new(id.clone()));
        let reduced = app.beta_reduce_step().unwrap();
        assert_eq!(reduced, id);
    }

    #[test]
    fn test_debruijn_substitution() {
        // (λ.λ.1) 0 の簡約をテスト
        // K combinator: λ.λ.1 (これは λx.λy.x)
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        let a = DeBruijn::Var(0);

        // (λ.λ.1) 0 を1ステップ簡約
        let app = DeBruijn::App(Box::new(k), Box::new(a));
        let result = app.beta_reduce_step().unwrap();

        // 結果は λ.1 (aがシフトされて1になる)
        assert_eq!(result, DeBruijn::Abs(Box::new(DeBruijn::Var(1))));
    }

    #[test]
    fn test_debruijn_normalize() {
        // (λ.λ.1) 0 1 → 0
        // K combinator に2つの引数を適用
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        let a = DeBruijn::Var(0);
        let b = DeBruijn::Var(1);

        let expr = DeBruijn::App(
            Box::new(DeBruijn::App(Box::new(k), Box::new(a))),
            Box::new(b),
        );

        let normalized = expr.normalize(10);
        assert_eq!(normalized, DeBruijn::Var(0));
    }

    #[test]
    fn test_debruijn_s_combinator() {
        // S combinator: λ.λ.λ.2 0 (1 0)
        let s = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(
            DeBruijn::App(
                Box::new(DeBruijn::App(
                    Box::new(DeBruijn::Var(2)),
                    Box::new(DeBruijn::Var(0)),
                )),
                Box::new(DeBruijn::App(
                    Box::new(DeBruijn::Var(1)),
                    Box::new(DeBruijn::Var(0)),
                )),
            ),
        ))))));

        // Displayの実装は括弧を最小限にする
        assert_eq!(format!("{}", s), "\\a. \\b. \\c. a c (b c)");
    }

    #[test]
    fn test_debruijn_node_count() {
        // λ.0 のノード数は 2 (Abs + Var)
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        assert_eq!(id.node_count(), 2);

        // λ.λ.1 のノード数は 3 (Abs + Abs + Var)
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        assert_eq!(k.node_count(), 3);

        // (λ.0) 0 のノード数は 4 (App + Abs + Var + Var)
        let app = DeBruijn::App(Box::new(id), Box::new(DeBruijn::Var(0)));
        assert_eq!(app.node_count(), 4);
    }

    #[test]
    fn test_debruijn_complex_reduction() {
        // ((λ.λ.1) (λ.0)) (λ.0) → λ.0
        // K I I → I
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        let i = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));

        let expr = DeBruijn::App(
            Box::new(DeBruijn::App(Box::new(k), Box::new(i.clone()))),
            Box::new(i.clone()),
        );

        let normalized = expr.normalize(10);
        assert_eq!(normalized, i);
    }

    #[test]
    fn test_debruijn_shift() {
        // λ.0 を外側にシフト
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let shifted = id.shift(1, 0);
        // cutoff=0、Absで深さが1増えるので、内部ではcutoff=1になる
        // Var(0)は0 < 1なのでシフトされない
        assert_eq!(shifted, DeBruijn::Abs(Box::new(DeBruijn::Var(0))));
    }

    #[test]
    fn test_debruijn_equality() {
        // 同じ構造は等しい
        let id1 = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let id2 = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        assert_eq!(id1, id2);

        // 異なる構造は等しくない
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));
        assert_ne!(id1, k);
    }

    #[test]
    fn test_debruijn_hash() {
        use std::collections::HashSet;

        // HashSetで重複を検出できる
        let mut set = HashSet::new();
        let id1 = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let id2 = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));

        set.insert(id1.clone());
        set.insert(id2);

        // 同じ構造なので1つだけ
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_debruijn_eta_reduction() {
        // λ.(f 0) → f のテスト
        // f = Var(1) の場合
        let f = DeBruijn::Var(1);
        let app = DeBruijn::App(Box::new(f), Box::new(DeBruijn::Var(0)));
        let eta = DeBruijn::Abs(Box::new(app));

        let reduced = eta.eta_reduce_step().unwrap();
        // 結果は f を1段階シフトダウンしたもの = Var(0)
        assert_eq!(reduced, DeBruijn::Var(0));
    }

    #[test]
    fn test_debruijn_eta_reduction_no_reduce() {
        // λ.(0 0) → λ.(0 0) （0が自由に現れているので簡約されない）
        let var0 = DeBruijn::Var(0);
        let app = DeBruijn::App(Box::new(var0.clone()), Box::new(DeBruijn::Var(0)));
        let eta = DeBruijn::Abs(Box::new(app));

        // 0が自由に現れているので簡約できない
        assert!(eta.eta_reduce_step().is_none());
    }

    #[test]
    fn test_debruijn_normalize_with_eta() {
        // λ.(λ.0 0) → λ.0
        let id = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        let id_app = DeBruijn::App(Box::new(id), Box::new(DeBruijn::Var(0)));
        let eta = DeBruijn::Abs(Box::new(id_app));

        let normalized = eta.normalize(10);
        let expected = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        assert_eq!(normalized, expected);
    }

    #[test]
    fn test_debruijn_k_combinator_direct() {
        // Direct test: K = λ.λ.1 (which is \x.\y.x in normal form)
        let k = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))));

        // I = λ.0 (which is \x.x)
        let i = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));

        // K I = (λ.λ.1) (λ.0)
        let app = DeBruijn::App(Box::new(k.clone()), Box::new(i.clone()));

        // After beta reduction: substitute 0 with I in (λ.1)
        // Body is λ.1, replace 0 with I
        // Result should be λ.I (with I shifted up by 1)
        // I = λ.0, shifted up by 1 = λ.1
        // So result is λ.λ.1 which is K itself

        let reduced = app.beta_reduce_step();
        println!("K I reduced once: {:?}", reduced);

        if let Some(once_reduced) = reduced {
            // After one step, we should have λ.λ.1 (K itself)
            // Let's verify by normalizing further
            let normalized = once_reduced.clone().normalize(10);
            println!("K I normalized: {:?}", normalized);

            // The result should equal K because:
            // After beta: λ.I = λ.(λ.0) which needs to be eta-reduced
            // But λ.(λ.0) applied to something becomes (λ.0) which is I
        }
    }
}
