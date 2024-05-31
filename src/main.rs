use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use colored::Colorize;
use passwords::{analyzer, scorer, PasswordGenerator};
use rand_core::OsRng;
use rayon::prelude::*;
use zeroize::Zeroize;

mod errors;
use errors::{ArgonError, MyError};

// Configuration Constants
const MEMORY_COST: u32 = 50;
const TIME_COST: u32 = 2;
const PARALLELISM: u32 = 2;
const OUTPUT_LEN: usize = 32;

type PasswordWithScore = (String, f64);

fn main() -> Result<(), MyError> {
    let passwords = generate_passwords_using_rayon(16, 16)?;
    let argon2 = create_argon2();

    let results: Result<Vec<_>, _> = passwords
        .into_par_iter()
        .map(|(mut password, _)| {
            let salt = SaltString::generate(&mut OsRng);
            let result = hash_password(&argon2, &password, &salt);
            password.zeroize(); // Zeroize the password to prevent memory-based attacks
            result
        })
        .collect();

    match results {
        Ok(hashes) => {
            hashes
                .into_par_iter()
                .for_each(|hash| println!("Hash output: {}", hash));
            println!(
                "{}",
                "[LOG] All passwords have been hashed successfully".green()
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn create_argon2() -> Argon2<'static> {
    let params = Params::new(MEMORY_COST, TIME_COST, PARALLELISM, Some(OUTPUT_LEN))
        .expect("Failed to set Argon2 parameters");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn hash_password(
    argon2: &Argon2<'_>,
    password: &str,
    salt: &SaltString,
) -> Result<String, MyError> {
    argon2
        .hash_password(password.as_bytes(), salt)
        .map_err(|source| MyError::HashingError {
            source: ArgonError(source),
            salt: salt.clone(),
        })
        .map(|hash| hash.to_string())
}

fn generate_password(password_gen: &PasswordGenerator) -> Result<PasswordWithScore, MyError> {
    let password = password_gen
        .generate_one()
        .map_err(|_| MyError::PasswordGenerationError)?;
    let analyzed = analyzer::analyze(&password);
    let score = scorer::score(&analyzed);
    Ok((password, score))
}

fn generate_passwords_using_rayon(
    number_of_passwords: usize,
    length: usize,
) -> Result<Vec<PasswordWithScore>, MyError> {
    let pg = PasswordGenerator {
        length,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    };

    (0..number_of_passwords)
        .into_par_iter()
        .map(|_| generate_password(&pg))
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORDGENERATOR: PasswordGenerator = PasswordGenerator {
        length: 16,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    };

    #[test]
    fn test_generate_password() {
        match generate_password(&PASSWORDGENERATOR) {
            Ok((password, _score)) => {
                assert!(password.len() == 16 && password.chars().all(|c| c.is_ascii_graphic()));
            }
            Err(e) => {
                panic!("Password generation failed with error: {}", e);
            }
        }
    }

    #[test]
    fn test_hash_password() {
        let (mut password, _score) = match generate_password(&PASSWORDGENERATOR) {
            Ok(result) => result,
            Err(e) => panic!("Password generation failed with error: {}", e),
        };

        let argon2 = create_argon2();
        let salt = SaltString::generate(&mut OsRng);
        let result = argon2.hash_password(&password.as_bytes(), &salt);
        assert!(result.is_ok());
        password.zeroize();
    }
}
