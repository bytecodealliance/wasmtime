#define HIDE_FROM_CHECKER(x) x

class SomeClass {
public:
  static int MainDefinedFunction();
  static int SatelliteFunction(int x);
};

inline void debug_break() {}
