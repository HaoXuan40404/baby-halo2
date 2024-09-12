use std::{marker::PhantomData};

use halo2_proofs:: {arithmetic::Field, circuit::*, plonk::*, poly::Rotation};

#[derive(Clone, Debug)]
struct FiboConfig {
    pub advice: [Column<Advice>; 3],
    pub selector: Selector,
    pub instance: Column<Instance>,
}

struct FiboChip<F: Field> {
    config: FiboConfig,
    // not use
    _marker: PhantomData<F>,
}

impl<F: Field> FiboChip<F> {
    fn consturct(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FiboConfig {
        let col_a: Column<Advice> = meta.advice_column();
        let col_b: Column<Advice> = meta.advice_column();
        let col_c: Column<Advice> = meta.advice_column();
        let selector : Selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        // 限制每一行的col_a, col_b, col_c的值 col_a + col_b = col_c, s 启用selector
        meta.create_gate("add", |meta| {
            let s: Expression<F> = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            vec![s * (a + b - c)]

        });

        return FiboConfig {
            advice: [col_a, col_b, col_c],
            selector,
            instance,
        }
    }

    #[allow(clippy::type_complexity)]
    fn assign_first_row(&self, mut layouter: impl Layouter<F>) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error>{
        layouter.assign_region(|| "first row", 
        |mut region| {
            self.config.selector.enable(&mut region, 0)?;

            let a_cell = region.assign_advice_from_instance(
                || "a", 
            self.config.instance, 
            0, 
            self.config.advice[0],
            0
            )?;

            let b_cell = region.assign_advice_from_instance(
                || "b", 
                self.config.instance, 
            0, 
            self.config.advice[1],
            0
            )?;

            let c_cell = region.assign_advice(
                || "c", 
            self.config.advice[2],
            0,
            || a_cell.value().copied() + b_cell.value().copied()
            )?;

            Ok((a_cell, b_cell, c_cell))
        })
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, prev_b: &AssignedCell<F, F>, prev_c: &AssignedCell<F, F>) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(|| "next row", |mut region| {
            self.config.selector.enable(&mut region, 0)?;

            prev_b.copy_advice(|| "a", &mut region, self.config.advice[0], 0)?;
            prev_c.copy_advice(|| "b", &mut region, self.config.advice[1], 0)?;

            let c_cell = region.assign_advice(
                || "c",
                self.config.advice[2],
                0,
                || prev_b.value().copied() + prev_c.value().copied(),
            )?;

            Ok(c_cell)
        })
    }

    fn expose_public_input(&self, mut layouter: impl Layouter<F>, cell: &AssignedCell<F, F>, offset: usize) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, offset)
    }
    
}

#[derive(Default)]
struct MyCircuit<F>(PhantomData<F>);

impl<F: Field> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FiboChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FiboChip::consturct(config);

        let (_, mut prev_b, mut prev_c) = chip.assign_first_row(layouter.namespace(|| "first row"))?;

        for _i in 3..11 {
            let c_cell = chip.assign_row(layouter.namespace(|| "next row"), 
            &prev_b, &prev_c)?;

            prev_b = prev_c;
            prev_c = c_cell;
        }

        chip.expose_public_input(layouter.namespace(|| "output"), &prev_c, 2)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use halo2_proofs::{dev::MockProver, pasta::Fp};

    use super::MyCircuit;

    #[test]
    fn test_mock_proof() {
        let k = 4;
        let a = Fp::from(1);
        let b = Fp::from(1);
        let out = Fp::from(89);

        let circuit = MyCircuit(PhantomData);

        let public_input = vec![a, b, out];

        let prover = MockProver::run(k, &circuit, vec![public_input]).expect("proving failed");
        prover.assert_satisfied();

    }
}