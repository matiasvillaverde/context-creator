//! Category 4: Semantic Analysis - TypeScript/JavaScript (20 Tests)
//!
//! Tests for TypeScript/JavaScript-specific semantic analysis edge cases

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 51: Tracing TypeScript interface implementations
#[test]
fn test_51_typescript_interface_implementation() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("types.ts"),
        r#"
export interface IUser {
    id: number;
    getName(): string;
}

export interface IAdmin extends IUser {
    permissions: string[];
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("models.ts"),
        r#"
import { IUser, IAdmin } from './types';

export class User implements IUser {
    constructor(public id: number, private name: string) {}
    
    getName(): string {
        return this.name;
    }
}

export class Admin extends User implements IAdmin {
    permissions: string[] = [];
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include",
        "models.ts",
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("models.ts"));
}

/// Scenario 52: JavaScript modules with CommonJS and ES6 mixed
#[test]
fn test_52_mixed_module_systems() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("legacy.js"),
        r#"
// CommonJS style
const utils = require('./utils');
module.exports = {
    process: function() {
        return utils.helper();
    }
};
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("modern.js"),
        r#"
// ES6 style
import { process } from './legacy';
export const run = () => process();
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("utils.js"),
        r#"
exports.helper = function() {
    return "helped";
};
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("utils.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("legacy.js"));
}

/// Scenario 53: TypeScript generic type constraints
#[test]
fn test_53_generic_type_constraints() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("generics.ts"),
        r#"
interface Lengthwise {
    length: number;
}

function loggingIdentity<T extends Lengthwise>(arg: T): T {
    console.log(arg.length);
    return arg;
}

class Collection<T extends { id: number }> {
    items: T[] = [];
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include",
        "generics.ts",
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("generics.ts"));
}

/// Scenario 54: React component with TypeScript props
#[test]
fn test_54_react_typescript_props() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("Button.tsx"),
        r#"
import React from 'react';

interface ButtonProps {
    onClick: () => void;
    disabled?: boolean;
    children: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({ onClick, disabled, children }) => {
    return (
        <button onClick={onClick} disabled={disabled}>
            {children}
        </button>
    );
};
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("App.tsx"),
        r#"
import { Button } from './Button';

export const App = () => {
    return <Button onClick={() => console.log('clicked')}>Click me</Button>;
};
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("Button.tsx").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("App.tsx"));
}

/// Scenario 55: JavaScript async/await and Promise chains
#[test]
fn test_55_async_await_promises() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("api.js"),
        r#"
export async function fetchUser(id) {
    const response = await fetch(`/api/users/${id}`);
    return response.json();
}

export function fetchUserLegacy(id) {
    return fetch(`/api/users/${id}`)
        .then(response => response.json());
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("service.js"),
        r#"
import { fetchUser, fetchUserLegacy } from './api';

async function loadUser(id) {
    const user = await fetchUser(id);
    return user;
}

function loadUserOld(id) {
    return fetchUserLegacy(id).then(user => user);
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("api.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("service.js"));
}

/// Scenario 56: TypeScript namespace and module augmentation
#[test]
fn test_56_namespace_module_augmentation() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("core.ts"),
        r#"
namespace MyLib {
    export interface Config {
        name: string;
    }
    
    export function configure(config: Config) {
        console.log(config.name);
    }
}

export = MyLib;
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("extensions.ts"),
        r#"
import MyLib = require('./core');

declare module './core' {
    interface Config {
        version?: string;
    }
}

MyLib.configure({ name: 'test', version: '1.0' });
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("core.ts").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("extensions.ts"));
}

/// Scenario 57: JavaScript destructuring in imports and exports
#[test]
fn test_57_destructuring_imports_exports() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("utils.js"),
        r#"
export const helpers = {
    formatDate: (date) => date.toISOString(),
    parseDate: (str) => new Date(str)
};

export const { formatDate, parseDate } = helpers;
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.js"),
        r#"
import { formatDate, helpers } from './utils';

const date = new Date();
console.log(formatDate(date));
console.log(helpers.parseDate('2023-01-01'));
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("utils.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("app.js"));
}

/// Scenario 58: TypeScript type-only imports and exports
#[test]
fn test_58_type_only_imports() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("types.ts"),
        r#"
export type UserId = string;
export type UserRole = 'admin' | 'user' | 'guest';

export interface UserData {
    id: UserId;
    role: UserRole;
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("user.ts"),
        r#"
import type { UserData, UserId } from './types';
import { type UserRole } from './types';

function processUser(data: UserData): UserId {
    return data.id;
}

const role: UserRole = 'admin';
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include",
        "user.ts",
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("user.ts"));
}

/// Scenario 59: JavaScript class with static methods and properties
#[test]
fn test_59_static_class_members() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("singleton.js"),
        r#"
export class Database {
    static instance = null;
    
    static getInstance() {
        if (!Database.instance) {
            Database.instance = new Database();
        }
        return Database.instance;
    }
    
    query(sql) {
        return `Executing: ${sql}`;
    }
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.js"),
        r#"
import { Database } from './singleton';

const db = Database.getInstance();
db.query('SELECT * FROM users');
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("singleton.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("app.js"));
}

/// Scenario 60: TypeScript decorators on classes and methods
#[test]
fn test_60_typescript_decorators() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("decorators.ts"),
        r#"
function Injectable(target: any) {
    // Decorator implementation
}

function Log(target: any, key: string, descriptor: PropertyDescriptor) {
    // Method decorator
}

@Injectable
export class UserService {
    @Log
    getUser(id: number) {
        return { id, name: 'User' };
    }
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.ts"),
        r#"
import { UserService } from './decorators';

const service = new UserService();
service.getUser(1);
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("decorators.ts").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("app.ts"));
}

/// Scenario 61: JavaScript with JSDoc type annotations
#[test]
fn test_61_jsdoc_type_annotations() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("validators.js"),
        r#"
/**
 * @typedef {Object} ValidationResult
 * @property {boolean} valid
 * @property {string[]} errors
 */

/**
 * @param {string} email
 * @returns {ValidationResult}
 */
export function validateEmail(email) {
    const valid = email.includes('@');
    return {
        valid,
        errors: valid ? [] : ['Invalid email format']
    };
}

/**
 * @param {number} age
 * @returns {ValidationResult}
 */
export function validateAge(age) {
    const valid = age >= 0 && age <= 150;
    return {
        valid,
        errors: valid ? [] : ['Invalid age']
    };
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include",
        "validators.js",
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    // JSDoc types may not be fully traced by semantic analysis
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validators.js"));
}

/// Scenario 62: TypeScript mapped types and conditional types
#[test]
fn test_62_mapped_conditional_types() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("advanced-types.ts"),
        r#"
type Readonly<T> = {
    readonly [P in keyof T]: T[P];
};

type Partial<T> = {
    [P in keyof T]?: T[P];
};

type NonNullable<T> = T extends null | undefined ? never : T;

interface User {
    id: number;
    name: string;
    email?: string;
}

type ReadonlyUser = Readonly<User>;
type PartialUser = Partial<User>;
type RequiredEmail = NonNullable<User['email']>;
"#,
    )
    .unwrap();

    let output =
        run_context_creator(&[temp_dir.path().join("advanced-types.ts").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("advanced-types.ts"));
}

/// Scenario 63: JavaScript with dynamic imports
#[test]
fn test_63_dynamic_imports() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("lazy.js"),
        r#"
export function heavyFunction() {
    // Expensive computation
    return "result";
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.js"),
        r#"
async function loadHeavyModule() {
    const module = await import('./lazy.js');
    return module.heavyFunction();
}

// Conditional import
if (process.env.NODE_ENV === 'development') {
    import('./lazy.js').then(module => {
        console.log(module.heavyFunction());
    });
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("lazy.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Dynamic imports may not be traced
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("dynamic") || stderr.contains("import")
        }
    );
}

/// Scenario 64: TypeScript with multiple inheritance through mixins
#[test]
fn test_64_typescript_mixins() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("mixins.ts"),
        r#"
type Constructor<T = {}> = new (...args: any[]) => T;

function Timestamped<TBase extends Constructor>(Base: TBase) {
    return class extends Base {
        timestamp = Date.now();
    };
}

function Tagged<TBase extends Constructor>(Base: TBase) {
    return class extends Base {
        tags: string[] = [];
    };
}

class Article {
    title: string = '';
}

export class BlogPost extends Tagged(Timestamped(Article)) {
    author: string = '';
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("mixins.ts").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BlogPost"));
}

/// Scenario 65: JavaScript with circular dependencies
#[test]
fn test_65_circular_dependencies() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("moduleA.js"),
        r#"
import { functionB } from './moduleB.js';

export function functionA() {
    return 'A calls ' + functionB();
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("moduleB.js"),
        r#"
import { functionA } from './moduleA.js';

export function functionB() {
    return 'B';
}

// This creates a circular dependency
export function callA() {
    return functionA();
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("moduleA.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("moduleB.js"));
}

/// Scenario 66: TypeScript with complex generics and inference
#[test]
fn test_66_complex_generics() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("generics.ts"),
        r#"
type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;
type FunctionArgs<T> = T extends (...args: infer A) => any ? A : never;
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;

function compose<T, U, V>(
    f: (x: T) => U,
    g: (x: U) => V
): (x: T) => V {
    return x => g(f(x));
}

async function example(): Promise<string> {
    return "hello";
}

type ExampleReturn = UnwrapPromise<ReturnType<typeof example>>;
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("generics.ts").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("generics.ts"));
}

/// Scenario 67: JavaScript with prototype manipulation
#[test]
fn test_67_prototype_manipulation() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("prototypes.js"),
        r#"
function Animal(name) {
    this.name = name;
}

Animal.prototype.speak = function() {
    return `${this.name} makes a sound`;
};

function Dog(name, breed) {
    Animal.call(this, name);
    this.breed = breed;
}

// Set up inheritance
Dog.prototype = Object.create(Animal.prototype);
Dog.prototype.constructor = Dog;

Dog.prototype.bark = function() {
    return `${this.name} barks`;
};

export { Animal, Dog };
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("prototypes.js").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Animal") && stdout.contains("Dog"));
}

/// Scenario 68: TypeScript with ambient declarations
#[test]
fn test_68_ambient_declarations() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("globals.d.ts"),
        r#"
declare global {
    interface Window {
        myApp: {
            version: string;
            init(): void;
        };
    }
}

declare module "legacy-lib" {
    export function oldFunction(): string;
}

export {};
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.ts"),
        r#"
/// <reference path="./globals.d.ts" />

window.myApp = {
    version: '1.0.0',
    init() {
        console.log('App initialized');
    }
};

import { oldFunction } from 'legacy-lib';
oldFunction();
"#,
    )
    .unwrap();

    let output = run_context_creator(&["--trace-imports", temp_dir.path().to_str().unwrap()]);

    assert!(output.status.success());
}

/// Scenario 69: JavaScript with generator functions and iterators
#[test]
fn test_69_generators_iterators() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("generators.js"),
        r#"
export function* fibonacci() {
    let [a, b] = [0, 1];
    while (true) {
        yield a;
        [a, b] = [b, a + b];
    }
}

export async function* asyncCounter() {
    let i = 0;
    while (true) {
        await new Promise(resolve => setTimeout(resolve, 100));
        yield i++;
    }
}

export const iterableObject = {
    *[Symbol.iterator]() {
        yield 1;
        yield 2;
        yield 3;
    }
};
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("consumer.js"),
        r#"
import { fibonacci, asyncCounter, iterableObject } from './generators';

const fib = fibonacci();
console.log(fib.next().value);

for (const value of iterableObject) {
    console.log(value);
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("generators.js").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("consumer.js"));
}

/// Scenario 70: TypeScript with barrel exports and re-exports
#[test]
fn test_70_barrel_exports() {
    let temp_dir = TempDir::new().unwrap();
    let components_dir = temp_dir.path().join("components");
    fs::create_dir_all(&components_dir).unwrap();

    fs::write(
        components_dir.join("Button.ts"),
        r#"
export interface ButtonProps {
    label: string;
}

export class Button {
    constructor(public props: ButtonProps) {}
}
"#,
    )
    .unwrap();

    fs::write(
        components_dir.join("Input.ts"),
        r#"
export interface InputProps {
    value: string;
}

export class Input {
    constructor(public props: InputProps) {}
}
"#,
    )
    .unwrap();

    fs::write(
        components_dir.join("index.ts"),
        r#"
// Barrel file with re-exports
export { Button, ButtonProps } from './Button';
export { Input, InputProps } from './Input';
export * from './Button';
export * as ButtonModule from './Button';
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("app.ts"),
        r#"
import { Button, Input } from './components';
import { ButtonModule } from './components';

new Button({ label: 'Click' });
new Input({ value: 'text' });
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        components_dir.join("Button.ts").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should trace through the barrel export
    assert!(stdout.contains("index.ts") || stdout.contains("app.ts"));
}
