use pairing_plus::{bls12_381::*, CurveAffine, CurveProjective, Engine};
use ff_zeroize::{Field, PrimeField};
use super::basic::*;
use crate::algorithm::kzg;

pub fn gen(n: &usize) -> FCpp {
    let beta = Fr::from_repr(hash_to_field_repr("VC seed")).unwrap();

    let mut v_vec = Vec::with_capacity(*n); 
    let mut beta_square = beta;
    beta_square.mul_assign(&beta);
    let mut beta_power = Fr::one();
    for _ in 0..*n {
        beta_power.mul_assign(&beta_square); // compute beta^{2i}
        v_vec.push(G2Affine::one().mul(beta_power).into_affine());
    }

   FCpp {v_vec}
}
pub fn com(pp:&FCpp, a_vec:&Vec<G1Affine>)-> Fq12 {
   let com = Bls12::pairing_multi_product(&a_vec, &pp.v_vec);
   
   com
}

pub fn open(pp:FCpp, kzgpp: &KZGpp, i: &usize, a_vec:Vec<G1Affine>)-> PI {
    let n = pp.v_vec.len();
    let l:usize = (n as f64).log2() as usize;
    if *i >= n {
        panic!("Index out of bounds");
    }
    //b=(0,...,1,...,0)
    let mut b_loop:Vec<FrRepr> = vec![FrRepr([0, 0, 0, 0]); n];
    b_loop[*i] = FrRepr([1, 0, 0, 0]);

    let mut x_inverse_vec = Vec::with_capacity(l); 
    let mut l_vec = Vec::with_capacity(l); 
    let mut r_vec = Vec::with_capacity(l); 
    let mut a_loop = a_vec;
    let mut v_loop = pp.v_vec;
    let mut x_loop = FrRepr([0, 0, 0, 0]);
    let mut mid = n >> 1;
    for j in 1..l+1 {
        // let mut v_0 = v_loop[0].clone();
        // let mut a_1 = a_loop[1].clone();
        // println!("proof j:{:?}, mid:{:?}", j, mid);
        let a_left = a_loop[..mid].to_vec();
        let a_right = a_loop[mid..].to_vec();
        let v_left = v_loop[..mid].to_vec();
        let v_right = v_loop[mid..].to_vec();
        let b_left = b_loop[..mid].to_vec();
        let b_right = b_loop[mid..].to_vec();
        let b_left_u64: Vec<&[u64; 4]> = b_left.iter().map(|s| &s.0).collect();
        let b_right_u64: Vec<&[u64; 4]> = b_right.iter().map(|s| &s.0).collect();

        // println!("a_left:{:?}, a_right:{:?}", a_left.len(), a_right.len());
        // println!("v_left:{:?}, v_right:{:?}", v_left.len(), v_right.len());
        // println!("b_left:{:?}, b_right:{:?}", b_left.len(), b_right.len());
        //compute L_j and R_j
        let left1 = Bls12::pairing_multi_product(&a_right, &v_left);
        let left2 = G1Affine::sum_of_products(&a_right, &b_left_u64).into_affine();    
        l_vec.push((left1, left2));      
        let right1 = Bls12::pairing_multi_product(&a_left, &v_right);
        let right2 = G1Affine::sum_of_products(&a_left, &b_right_u64).into_affine();
        r_vec.push((right1, right2));

        // hash the values into scalars
        x_loop = hash_to_x(&x_loop, &left1, &left2, &right1, &right2);
        // println!("x_loop in proof: {:?}", x_loop);
        let x_loop_inverse = Fr::from_repr(x_loop).unwrap().inverse().unwrap();
        x_inverse_vec.push(x_loop_inverse);
        
        // let mut a_1_exp: G1Affine = a_1.mul(x_loop).into();
        // let mut v_0_exp: G2Affine = v_0.mul(x_loop).into();
        // let mut result1 = Bls12::pairing(a_1_exp, v_0);
        // let mut result2 = Bls12::pairing(a_1, v_0_exp);
        // println!("result1: {:?}", result1);
        // println!("result2: {:?}", result2);
        
        let mut a_right_exp: Vec<G1Affine> = vec![G1Affine::one(); mid];
        a_loop.clear();
        a_loop.extend(a_left.to_vec().clone());
        let mut v_right_exp: Vec<G2Affine> = vec![G2Affine::one(); mid];
        v_loop.clear();
        v_loop.extend(v_left.to_vec().clone());
        let mut b_right_exp:Vec<FrRepr> = vec![FrRepr([0, 0, 0, 0]); mid];
        b_loop.clear();
        b_loop.extend(b_left.to_vec().clone());
        for k in 0..mid{
            a_right_exp[k] = a_right[k].mul(x_loop).into();
            let mut tmp1 = G1::from(a_left[k]);
            tmp1.add_assign(&G1::from(a_right_exp[k]));
            a_loop[k] = tmp1.into();  

            v_right_exp[k] = v_right[k].mul(x_loop_inverse).into();
            let mut tmp2 = G2::from(v_left[k]);
            tmp2.add_assign(&G2::from(v_right_exp[k]));
            v_loop[k] = tmp2.into(); 

            let mut tmp3 = Fr::from_repr(b_right[k].into()).unwrap();
            tmp3.mul_assign(&x_loop_inverse);
            b_right_exp[k] = tmp3.into();
            let mut tmp4 = Fr::from_repr(b_left[k].into()).unwrap();
            tmp4.add_assign(&Fr::from_repr(b_right_exp[k]).unwrap());
            b_loop[k] = tmp4.into();
        } 
        mid = mid >> 1;   
    }
    
    let finalA = a_loop[0];
    let finalv = v_loop[0];

    let mut poly = vec![x_inverse_vec[0]; 2*n+1]; 
    let point = Fr::from_repr(hash_to_field_repr("KZG point")).unwrap();
    let finalv_proof = kzg::open(kzgpp, &poly, point).into_affine();
    PI {
        l_vec,
        r_vec,
        finalA, 
        finalv,
        finalv_proof,
    }
 }

 pub fn verify(pp:&FCpp, kzgpp: &KZGpp, com:Fq12, i:&usize, a_i: G1Affine, pi:&PI)-> bool {
    
    let n = pp.v_vec.len();
    let l:usize = (n as f64).log2() as usize;
    if *i >= n {
        panic!("Index out of bounds");
    }
    let i_binary_vec = to_fixed_length_binary_vec(i, &l);
    
    let c_loop1 = com;
    let c_loop2 = G1::from(a_i);
    let mut c_loop = (c_loop1, c_loop2);
    let mut x_loop:FrRepr = FrRepr([0, 0, 0, 0]);
    let mut x_inverse_vec = Vec::with_capacity(l); 

    let mut final_b:Fr = Fr::one();
    for j in 1..l+1 {
        let (left1, left2) = pi.l_vec[j-1];
        let (right1, right2) = pi.r_vec[j-1];
        x_loop = hash_to_x(&x_loop, &left1, &left2, &right1, &right2);
        // println!("x_loop in verification: {:?}", x_loop);
        let x_loop_inverse = Fr::from_repr(x_loop).unwrap().inverse().unwrap();
        x_inverse_vec.push(x_loop_inverse);

        let left1_exp = left1.pow(x_loop);
        // println!("left1_pow: {:?}", left1_exp);

        let x_loop_inverse1:FrRepr = x_loop_inverse.into();
        let right1_exp = right1.pow(x_loop_inverse1);

        c_loop.0.mul_assign(&left1_exp);
        c_loop.0.mul_assign(&right1_exp);

        let left2_exp = left2.mul(x_loop);
        let right2_exp = right2.mul(x_loop_inverse);
        c_loop.1.add_assign(&left2_exp);
        c_loop.1.add_assign(&right2_exp);

        if i_binary_vec[j-1] == 1{
            final_b.mul_assign(&x_loop_inverse);
        }
    }

    let point = Fr::from_repr(hash_to_field_repr("KZG point")).unwrap();
    let mut value = Fr::zero();
    let mut x = Fr::one();
    for i in 0..l {
        x.mul_assign(&point); // compute point^{i}
        let mut coffi = x_inverse_vec[i];
        coffi.mul_assign(&x);
        value.mul_assign(&coffi);
    }
    kzg::verify(kzgpp, point, value, pi.finalv, pi.finalv_proof);


    let r1:bool = c_loop.0 == Bls12::pairing(pi.finalA, pi.finalv);
    let r2:bool = c_loop.1 == pi.finalA.mul(final_b).into();

    // println!("r1: {:?}, r2: {:?}", r1, r2);
    r1 && r2
 }


