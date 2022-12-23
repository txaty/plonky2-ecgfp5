use crate::gadgets::base_field::{CircuitBuilderQuinticExt, QuinticExtensionTarget};
use num::{BigUint, FromPrimitive, Zero};
use plonky2::{field::extension::quintic::QuinticExtension, iop::target::Target};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::ops::Square;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::BoolTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_ecdsa::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint};

fn curve_a<F: RichField + Extendable<5>>() -> QuinticExtension<F> {
    let a = QuinticExtension::<F>::from_canonical_u16(2);
    let b = QuinticExtension::<F>::from_canonical_u16(263);
    let three = QuinticExtension::<F>::from_canonical_u16(3);

    (three * b - a.square()) / three
}

pub fn scalar_field_order() -> BigUint {
	let mut res = BigUint::from_u128(25 * 5 * 163 * 769 * 1059871).unwrap();
	res *= BigUint::from_u128(253243826720162431254857814100127).unwrap();

	let big_factor_limbs = [
		198400523,
		053184002,
		814403536,
		918162724,
		916343842,
		520561,
	];
	let big_factor = big_factor_limbs.into_iter().rev().enumerate().fold(BigUint::zero(), |acc, (i, limb)| ((BigUint::from_u64(100_000_000).unwrap() * acc) + BigUint::from_i32(limb).unwrap()));
	res * big_factor
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct CurveTarget(([QuinticExtensionTarget; 2], BoolTarget));

pub trait CircuitBuilderCurve<F: RichField + Extendable<5>> {
    fn add_virtual_curve_target(&mut self) -> CurveTarget;
    fn curve_constant(
        &mut self,
        x: QuinticExtension<F>,
        y: QuinticExtension<F>,
        is_inf: bool,
    ) -> CurveTarget;
    fn curve_zero(&mut self) -> CurveTarget;
    fn curve_generator(&mut self) -> CurveTarget;
    fn curve_select(&mut self, cond: BoolTarget, a: CurveTarget, b: CurveTarget) -> CurveTarget;
    fn curve_random_access(&mut self, access_index: Target, v: Vec<CurveTarget>) -> CurveTarget;

    fn curve_add(&mut self, a: CurveTarget, b: CurveTarget) -> CurveTarget;
    fn curve_double(&mut self, a: CurveTarget) -> CurveTarget;
    fn curve_scalar_mul(&mut self, a: CurveTarget, scalar: BigUintTarget) -> CurveTarget;

    fn curve_encode_to_quintic_ext(&mut self, a: CurveTarget) -> QuinticExtensionTarget;
    fn curve_decode_from_quintic_ext(&mut self, a: QuinticExtensionTarget) -> CurveTarget;
}

impl<F: RichField + Extendable<5> + Extendable<D>, const D: usize> CircuitBuilderCurve<F>
    for CircuitBuilder<F, D>
{
    fn add_virtual_curve_target(&mut self) -> CurveTarget {
        let x = self.add_virtual_quintic_ext_target();
        let y = self.add_virtual_quintic_ext_target();
        let is_inf = self.add_virtual_bool_target_safe();
        CurveTarget(([x, y], is_inf))
    }

    fn curve_constant(
        &mut self,
        x: QuinticExtension<F>,
        y: QuinticExtension<F>,
        is_inf: bool,
    ) -> CurveTarget {
        let x = self.constant_quintic_ext(x);
        let y = self.constant_quintic_ext(y);
        let is_inf = self.constant_bool(is_inf);
        CurveTarget(([x, y], is_inf))
    }

    fn curve_zero(&mut self) -> CurveTarget {
        self.curve_constant(QuinticExtension::<F>::ZERO, QuinticExtension::<F>::ZERO, true)
    }

    fn curve_generator(&mut self) -> CurveTarget {
        let x = QuinticExtension::<F>::from_basefield_array(
            [
                F::from_noncanonical_u64(12883135586176881569),
                F::from_noncanonical_u64(4356519642755055268),
                F::from_noncanonical_u64(5248930565894896907),
                F::from_noncanonical_u64(2165973894480315022),
                F::from_noncanonical_u64(2448410071095648785)
            ]
        );

        let y = QuinticExtension::<F>::from_basefield_array(
            [
                F::from_noncanonical_u64(13835058052060938241),
                F::from_noncanonical_u64(0),
                F::from_noncanonical_u64(0),
                F::from_noncanonical_u64(0),
                F::from_noncanonical_u64(0)
            ]
        );

        self.curve_constant(x, y, false)
    }

    fn curve_select(&mut self, cond: BoolTarget, a: CurveTarget, b: CurveTarget) -> CurveTarget {
        let CurveTarget(([ax, ay], a_is_inf)) = a;
        let CurveTarget(([bx, by], b_is_inf)) = b;
        CurveTarget((
            [
                self.select_quintic_ext(cond, ax, bx),
                self.select_quintic_ext(cond, ay, by),
            ],
            BoolTarget::new_unsafe(self.select(cond, a_is_inf.target, b_is_inf.target)),
        ))
    }

    fn curve_random_access(&mut self, access_index: Target, v: Vec<CurveTarget>) -> CurveTarget {
        let xs = Vec::new();
        let ys = Vec::new();
        let is_infs = Vec::new();
        for CurveTarget(([x, y], is_inf)) in v {
            xs.push(x);
            ys.push(y);
            is_infs.push(is_inf.target);
        }

        CurveTarget((
            [
                self.random_access_quintic_ext(access_index, xs),
                self.random_access_quintic_ext(access_index, ys)
            ],
            BoolTarget::new_unsafe(self.random_access(access_index, is_infs))
        ))
    }

    fn curve_add(&mut self, a: CurveTarget, b: CurveTarget) -> CurveTarget {
        let CurveTarget(([x1, y1], a_is_inf)) = a;
        let CurveTarget(([x2, y2], b_is_inf)) = b;
        let three = QuinticExtension::<F>::from_canonical_u16(3);

        let sx = self.is_equal_quintic_ext(x1, x2);
        let sy = self.is_equal_quintic_ext(y1, y2);

        let lambda_0_if_sx_0 = self.sub_quintic_ext(y2, y1);
        let mut lambda_0_if_sx_1 = self.square_quintic_ext(x1);
        lambda_0_if_sx_0 = self.mul_const_quintic_ext(three, lambda_0_if_sx_0);
        lambda_0_if_sx_0 = self.add_const_quintic_ext(lambda_0_if_sx_0, curve_a());

        let lambda_1_if_sx_0 = self.add_quintic_ext(y1, y1);
        let mut lambda_1_if_sx_1 = self.sub_quintic_ext(x2, x1);

        // note: paper has a typo. select opposite what the paper says
        let lambda_0 = self.select_quintic_ext(sx, lambda_0_if_sx_0, lambda_0_if_sx_1);
        let lambda_1 = self.select_quintic_ext(sy, lambda_1_if_sx_0, lambda_1_if_sx_1);
        let lambda = self.div_quintic_ext(lambda_0, lambda_1);

        let mut x3 = self.square_quintic_ext(lambda);
        x3 = self.sub_quintic_ext(x3, x1);
        x3 = self.sub_quintic_ext(x3, x2);

        let mut y3 = self.sub_quintic_ext(x1, x3);
        y3 = self.mul_quintic_ext(lambda, y3);
        y3 = self.sub_quintic_ext(y3, y1);

        let c_is_inf = self.and(sx, sy);
        let c = CurveTarget(([x3, y3], c_is_inf));

        let sel = self.curve_select(a_is_inf, b, c);
        self.curve_select(b_is_inf, a, sel)
    }

    // TODO: optimize
    fn curve_double(&mut self, a: CurveTarget) -> CurveTarget {
        self.curve_add(a, a)
    }

    /// a: the point to multiply by
    /// b: little-endian bit representation of the scalar (i.e. least-significant first)
    fn curve_scalar_mul(&mut self, a: CurveTarget, scalar: BigUintTarget) -> CurveTarget {
        const WINDOW_BITS: usize = 4;
        let zero = self.zero();
        let one = self.one();
		let n = self.constant_biguint(&scalar_field_order());
		let max_digit = self.constant(F::from_canonical_u64(1 << WINDOW_BITS));

		let reduced_scalar = self.rem_biguint(&scalar, &n);
		let scalar = self.add_biguint(&reduced_scalar, &n);
        let mut scalar_bits = scalar.limbs.into_iter().flat_map(|limb| self.split_le(limb.0, 32)).collect::<Vec<_>>();

        let mut signs = Vec::new();
        let mut digits = Vec::new();
        let mut carry = self.constant_bool(false);
        for chunk in scalar_bits.chunks_exact(WINDOW_BITS) {
            let terms = (0..WINDOW_BITS-1).map(|i| self.mul_const(F::from_canonical_u32(1 << i), chunk[i].target));
            let lower_bits = self.add_many(terms);
            
            let mut d_if_lte_8 = self.mul_const_add(F::from_canonical_u32(1 << (WINDOW_BITS - 1)), chunk[WINDOW_BITS - 1].target, lower_bits);
            d_if_lte_8 = self.add(d_if_lte_8, carry.target);

            let mut d_if_gt_8 = self.sub(d_if_lte_8, max_digit);
            d_if_gt_8 = self.neg(d_if_gt_8);
            
            let lower_bits_zero = self.is_equal(lower_bits, zero);
            let lower_bits_not_zero =  BoolTarget::new_unsafe(self.sub(one, lower_bits_zero.target));

            // chunk on its own > 8
            let chunk_gt_8 = self.and(chunk[WINDOW_BITS - 1], lower_bits_not_zero);
            // chunk plus carry > 8
            let chunk_plus_carry_gt_8 = self.and(chunk[WINDOW_BITS - 1], carry);

            // chunk_gt_8 OR chunk_plus_carry_gt_8
            let gt_8 = self.mul(chunk_gt_8.target, chunk_plus_carry_gt_8.target);
            let gt_8 = self.mul_const(-F::ONE, gt_8);
            let gt_8 = BoolTarget::new_unsafe(self.add_many([
                gt_8,
                chunk_gt_8.target,
                chunk_plus_carry_gt_8.target
            ]));

            carry = BoolTarget::new_unsafe(self.select(gt_8, one, zero));
            digits.push(self.select(gt_8, d_if_gt_8, d_if_lte_8));
            signs.push(gt_8);
        }

        let mut window = vec![a];
        for _ in 0..(1 << (WINDOW_BITS - 1)) {
            let prev = window.last().unwrap();
            window.push(
                self.curve_add(*prev, a)
            );
        }

        let mut q = self.curve_zero();
        for (sign, magnitude) in signs.into_iter().zip(digits) {
            for _ in 0..WINDOW_BITS {
                q = self.curve_double(q);
            }

            let lookup_res = self.curve_random_access(magnitude, window);
            q = self.curve_add(q, lookup_res);
        }

        q
    }

    fn curve_encode_to_quintic_ext(&mut self, a: CurveTarget) -> QuinticExtensionTarget {
        let CurveTarget(([x, y], is_inf))  = a;
        let adiv3 = self.constant_quintic_ext(QuinticExtension::<F>::from_canonical_u16(2));
        let denom = self.sub_quintic_ext(adiv3, x);
        let w = self.div_quintic_ext(x, y);

        let zero = self.zero_quintic_ext();
        self.select_quintic_ext(is_inf, zero, w)
    }
}