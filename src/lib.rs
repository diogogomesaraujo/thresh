//! This crate contains all the functions to generate and share secret keys. The secret keys' bit size can be ajusted by changing the const.

use rug::{rand::RandState, Integer};

mod modular;

/// Const value of the Integers' size in bits.
pub const BITS: u32 = 256;

/// Function that calculates the biggest prime for the number of bits available (if needed).
pub fn calculate_biggest_prime(rnd: &mut RandState) -> Integer {
    loop {
        let candidate = Integer::from(Integer::random_bits(BITS, rnd));
        match candidate.is_probably_prime(30) {
            rug::integer::IsPrime::No => continue,
            _ => {
                return candidate;
            }
        }
    }
}

/// Function that calculates the y value for a given polinomial and an x value.
pub fn calculate_y(x: &Integer, pol: &[Integer], prime: &Integer) -> Integer {
    pol.iter().enumerate().fold(Integer::ZERO, |acc, (i, p)| {
        modular::add(
            acc,
            modular::mul(p.clone(), modular::pow(x, &Integer::from(i), prime), prime),
            prime,
        )
    })
}

/// Function that calculates the lagrange polinomial (it is the algorythm used to recover a secret).
pub fn lagrange_pol(x: &Integer, pol: &[(Integer, Integer)], prime: &Integer) -> Integer {
    let n = pol.len();
    let mut result = Integer::from(0);

    for i in 0..n {
        let (xi, yi) = pol[i].clone();

        let mut num = Integer::from(1);
        let mut den = Integer::from(1);

        for j in 0..n {
            if j != i {
                let (xj, _) = pol[j].clone();
                num = modular::mul(
                    num.clone(),
                    modular::sub(Integer::from(x), Integer::from(&xj), prime),
                    prime,
                );
                den = modular::mul(den, modular::sub(xi.clone(), xj, prime), prime);
            }
        }
        let div = modular::div(num, den, prime);
        let term = modular::mul(yi, div, prime);

        result = modular::add(result, term, prime);
    }

    result
}

/// Function that generates a unique number for a given vector.
pub fn generate_unique(rnd: &mut RandState, v: &[Integer]) -> Integer {
    let r = Integer::from(Integer::random_bits(BITS, rnd));

    match v.iter().find(|&i| i == &r) {
        Some(_) => generate_unique(rnd, v),
        None => r,
    }
}

/// Function that generates a polinomial (it is used to divide the secret into multiple shares).
pub fn generate_pol(key: Integer, k: u64, rnd: &mut RandState) -> Vec<Integer> {
    let mut pol: Vec<Integer> = vec![key];

    for _i in 1..k {
        let r = generate_unique(rnd, &pol);
        pol.push(r);
    }

    pol
}

/// Function that creates the secret shares.
pub fn create_secret_shares(
    key: Integer,
    k: u64,
    n: u64,
    prime: &Integer,
) -> Vec<(Integer, Integer)> {
    let mut rnd = RandState::new();

    let pol = generate_pol(key, k, &mut rnd);
    let mut shares: Vec<(Integer, Integer)> = Vec::new();
    let mut xs = Vec::new();

    for _i in 0..n {
        let x = generate_unique(&mut rnd, &xs);
        xs.push(x.clone());

        let y = calculate_y(&x, &pol, prime);
        shares.push((x, y));
    }

    shares
}

/// Function that generates the secret key (if needed).
pub fn generate_key(rnd: &mut RandState, prime: &Integer) -> Integer {
    Integer::from(Integer::random_bits(BITS, rnd)).modulo(&prime)
}

/// Function that recovers the secret.
pub fn recover_secret(shares: &[(Integer, Integer)], prime: &Integer) -> Integer {
    lagrange_pol(&Integer::from(0), shares, prime)
}

/// Bulk test for the Shamir Secret Sharing library using randomly generated numbers.
#[test]
fn test_create_recover_bulk() {
    let mut handles = Vec::new();

    for _i in 0..5 {
        let handle = std::thread::spawn(|| {
            let mut rnd = RandState::new();
            let prime = calculate_biggest_prime(&mut rnd);

            for _i in 0..200000 {
                let key = generate_key(&mut rnd, &prime);
                let k = 2;
                let n = 3;

                let shares = create_secret_shares(key.clone(), k, n, &prime);
                let subset = &shares[0..(k as usize)];

                let recovered_key = recover_secret(subset, &prime);

                assert_eq!(
                    key, recovered_key,
                    "Secret Shares: {:?} \n{key} compared to {recovered_key}\n",
                    shares
                );
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
