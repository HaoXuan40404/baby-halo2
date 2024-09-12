use std::{marker::PhantomData};

use halo2_proofs:: {arithmetic::Field, circuit::*, plonk::*, poly::Rotation};

// #[derive(Clone, Debug)]
// struct ACell<F: Field>(AssignedCell<F, F>);

#[derive(Clone, Debug)]
struct FiboConfig {
    pub advice: Column<Advice>,
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
        let advice: Column<Advice> = meta.advice_column();
        let selector : Selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(advice);
        meta.enable_equality(instance);

        // 限制每一行的col_a, col_b, col_c的值 col_a + col_b = col_c, s 启用selector
        meta.create_gate("add", |meta| {
            let s: Expression<F> = meta.query_selector(selector);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            vec![s * (a + b - c)]

        });

        return FiboConfig {
            advice,
            selector,
            instance,
        }
    } 

    #[allow(clippy::type_complexity)]
    fn assign(&self, mut layouter: impl Layouter<F>, nrows: usize) -> Result< AssignedCell<F, F>, Error>{
        layouter.assign_region(|| "entire fibonacci table", 
        |mut region| {
            self.config.selector.enable(&mut region, 0)?;
            self.config.selector.enable(&mut region, 1)?;

            let mut a_cell = region.assign_advice_from_instance(
                || "1", 
            self.config.instance, 
            0, 
            self.config.advice,
            0
            )?;

            let mut b_cell = region.assign_advice_from_instance(
                || "1", 
                self.config.instance, 
            0, 
            self.config.advice,
            1
            )?;

            for row in 2..nrows {
                if row < nrows - 2 {
                    self.config.selector.enable(&mut region, row)?;
                }
                let c_val = a_cell.value().copied() + b_cell.value().copied();

                let c_cell = region.assign_advice(|| "advice", 
            self.config.advice,
            row, 
            || c_val)?;

                a_cell = b_cell;
                b_cell = c_cell;
            }
            

            Ok(b_cell)
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

        let out_cell = chip.assign(layouter.namespace(|| "entire table"), 11)?;

        chip.expose_public_input(layouter.namespace(|| "output"), &out_cell, 2)?;

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
        let k = 5;
        let a = Fp::from(1);
        let b = Fp::from(1);
        let out = Fp::from(89);

        let circuit = MyCircuit(PhantomData);

        let public_input = vec![a, b, out];

        let prover = MockProver::run(k, &circuit, vec![public_input]).expect("proving failed");
        prover.assert_satisfied();
    }
}