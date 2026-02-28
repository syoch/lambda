let
  # SKI
  S =
    x: y: z:
    x z (y z);
  K = a: _: a;
  I = a: a;

  #* Church bool
  BT = t: f: t;
  BF = t: f: f;

  #* Church number
  CP =
    x: y: s: z:
    x s (y s z);
  CM =
    x: y: s:
    x (y s);
  CD =
    n: s: z:
    n (x: b: b (x s)) (K z) I;
  CZ = n: n (K BF) BT;

  C0 = s: z: z;
  C1 = s: z: s z;
  C2 = s: z: s (s z);
  C4 = CP C2 C2;
  C6 = CP C2 C4;
  C7 = CP C1 C6;

  #* Pair
  PP =
    x: y: c:
    c x y;
  PF = p: p (x: y: x);
  PS = p: p (x: y: y);

  #* List
  LN = f: z: z;
  LP =
    x: l: f: z:
    f x (l f z);
  LT = l: l (t: l: t) LN;
  LR = l: l (t: l: l) LN;

  #* Fix Combinator
  Z = f: (x: f (x x)) (x: f (x x));

  #* User
  DecL = n: CZ n LN (LP n (DecL (CD n)));
  FactImpl = self: n: CZ n C1 (CM n (self (CD n)));
  Fact = Z FactImpl;

  #* Utility
  ToInt = v: v (x: x + 1) 0;
  ToBool = b: b true false;
  ToArray = l: l (t: l: [ t ] ++ l) [ ];

  L = DecL C6;
in
# ToInt (Fact C6)
map ToInt (ToArray L)
