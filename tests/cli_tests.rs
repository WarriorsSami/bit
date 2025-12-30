mod add;
mod commit;

mod branch;
mod checkout;
/// Contains common utilities and setup boilerplate, such as
/// 1. Scaffolding temp git repo
/// 2. Creating files with random content
/// 3. Running bit commands
/// 4. Running git commands
/// 5. Comparing index contents
mod common;
mod diff;
mod hash_object;
mod init;
mod log;
mod ls_tree;
mod status;
