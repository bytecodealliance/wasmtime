import js from "@eslint/js";

export default [
  js.configs.recommended,

  {
    rules: {
      "no-unused-vars": "error",
      "no-undef": "error",
    },
  },
];
