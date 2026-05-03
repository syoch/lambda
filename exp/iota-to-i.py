import sys


type Expr = str | tuple[Expr, Expr]


def take_iota_expr(s: str) -> tuple[str, Expr]:
    s = s.strip()
    if s[0] == "*":
        s = s[1:]
        s, term1 = take_iota_expr(s)
        s, term2 = take_iota_expr(s)
        return s, (term1, term2)
    else:
        return s[1:], s[0]


def take_unlambda_expr(s: str) -> tuple[str, Expr]:
    s = s.strip()
    if s[0] == "`":
        s = s[1:]
        s, term1 = take_unlambda_expr(s)
        s, term2 = take_unlambda_expr(s)
        return s, (term1, term2)
    elif s[0] == "[":
        end_idx = s.find("]")
        if end_idx == -1:
            raise ValueError("unmatched [ in unlambda expression")
        return s[end_idx + 1 :], s[1:end_idx]
    else:
        return s[1:], s[0]


def show_as_lambda(expr: Expr, is_left: bool = False) -> str:
    if isinstance(expr, str):
        if expr == "s":
            return "S"
        elif expr == "k":
            return "K"
        elif expr == "i":
            return "I"
        return expr
    else:
        app_expr = f"{show_as_lambda(expr[0], True)} {show_as_lambda(expr[1], False)}"
        if is_left:
            return app_expr
        else:
            return f"({app_expr})"


s = sys.argv[1]
while s:
    try:
        s, e = take_unlambda_expr(s.replace("\n", ""))
        print("-->", show_as_lambda(e))
    except Exception:
        print("Fin")
        print(s)
        break
    # print(s)
