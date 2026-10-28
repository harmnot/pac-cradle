# Code Style Guidelines

- always safe type and avoid using `any` type in TypeScript.
- better error handling with safe error handling patterns.
- avoid deeply nested code blocks, prefer early returns and guard clauses.
- avoid if else deeply nested, prefer match or early return.
- avoid 3 parameters in function, prefer to use object as parameter.
- make reusable functions to have single responsibility and avoid side effects.
- follow eslint and prettier rules for code formatting and consistency.
- always write documentation comments for functions, classes, and modules.
- avoid using magic numbers or strings, use constants or enums instead.
- avoid using `console.log` in production code, use proper logging libraries or error reporting tools.
- avoid using `any` type in TypeScript, prefer to use specific types or generics.
- avoid using `var`, prefer `let` or `const` for variable declarations.
- avoid using `==` or `!=`, prefer `===` or `!==` for comparisons.
- avoid using `for` loops, prefer `for...of` or array methods like `map`, `filter`, and `reduce`.
- avoid using `null` or `undefined` as values, prefer to use `Option` or `Maybe` types for optional values.
- always use `async/await` for asynchronous code, avoid using callbacks or promises directly.
- avoid using `eval` or `Function` constructor, prefer to use safer alternatives.
- always follow color schemes and design tokens defined in the project, avoid hardcoding colors or styles.
- always use semantic HTML elements and ARIA attributes for accessibility, avoid using non-semantic elements like `div` or `span` for layout or styling purposes.
- avoid using inline styles, prefer to use CSS classes or styled components for styling.
- the UI must High fidelity and follow the design system, avoid using custom styles or components that deviate from the design system.
- use existing components and libraries instead of creating new ones, avoid reinventing the wheel.
- if you need to create a new component, make sure it follows the design system and is reusable across the project.

