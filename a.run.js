const church_a = (x => x(x))(f => n => s => z => n ? s(f(f)(n - 1)(s)(z)) : z)(5);
const church_b = (x => x(x))(f => n => s => z => n ? s(f(f)(n - 1)(s)(z)) : z)(3);

const church_sub = (x => x(x))(f => m => n => s => z => m(f(f)(m - 1)(s)(z)) ? s(f(f)(m - 1)(s)(z)) : z);

const getChurchNumeral = n => n(m => m + 1)(0);
const getChurchBoolean = b => b(true)(false);
console.log(getChurchNumeral(church_a));