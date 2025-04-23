// Compile with:
//   clang++ --target=wasm32-wasi fraction-norm.cc -o fraction-norm.wasm -g \
//     -O0 -fdebug-prefix-map=$PWD=.

struct Fraction {
  long numerator;
  long denominator;
};

inline long abs(long x)
{
  return x >= 0 ? x : -x;  
}

extern "C"
void norm(Fraction &n)
{
  long a = abs(n.numerator), b = abs(n.denominator);
  if (a == 0 || b == 0) return;
  do {
   a %= b;
   if (a == 0) break;
   b %= a;
  } while (b > 0);
  long gcd = a + b;
  if (n.denominator > 0) {
    n.numerator /= gcd;
    n.denominator /= gcd;
  } else {
    n.numerator /= -gcd;
    n.denominator /= -gcd;
  }
}

int main()
{
  Fraction c = {6, 27};
  norm(c);
  return 0;
}
