use ordr::Worker;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let job = ordr::Job::builder()
        // .with_input(cells::IntracellularPart)
        .add::<cells::Nucleus>()
        .add::<cells::Spindle>()
        .build()
        .unwrap();

    let ctx = cells::Ctx::new();

    let mut worker = Worker::new(job, ctx);
    worker.run().unwrap();
    worker.wait_for_job().await.unwrap();
}

mod cells {
    use std::{sync::Arc, time::Duration};

    use rand::Rng;
    use tokio::{sync::Mutex, time::sleep};

    #[derive(Clone)]
    pub struct Ctx {
        index: Arc<Mutex<usize>>,
        nums: Vec<u64>,
    }

    impl Ctx {
        pub fn new() -> Self {
            Ctx::default()
        }

        async fn wait(&self) {
            let mut lock = self.index.lock().await;
            let index = *lock;
            *lock += 1;
            drop(lock);
            sleep(Duration::from_millis(self.nums[index])).await;
        }
    }

    impl Default for Ctx {
        fn default() -> Self {
            let mut rng = rand::rng();
            let mut nums = Vec::with_capacity(100);

            for _ in 0..100 {
                let num = rng.random_range(0..2000);
                nums.push(num);
            }

            Self {
                index: Arc::new(Mutex::new(0)),
                nums,
            }
        }
    }

    macro_rules! node {
        // No deps
        ($ident:ident: $ty:ident) => {
            #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
            pub struct $ty;
            #[ordr::producer]
            async fn $ident(c: ordr::Context<Ctx>) -> Result<$ty, ordr::Error> {
                c.state.wait().await;
                Ok($ty)
            }
        };
        ( $ident:ident: $ty:ident, $( $dep:ident ),* ) => {
            #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
            pub struct $ty;
            #[ordr::producer]
            async fn $ident(c: ordr::Context<Ctx>, $( _: $dep ),*) -> Result<$ty, ordr::Error> {
                c.state.wait().await;
                Ok($ty)
            }
        }
    }

    node!(cellular_component: CellularComponent);
    node!(cell: Cell, CellularComponent);
    node!(cell_part: CellPart, Cell);
    node!(cell_surface: CellSurface, CellPart);
    node!(intracellular: Intracellular, CellPart);
    node!(extracellular_region: ExtracellularRegion, CellularComponent);
    node!(extracellular_space: ExtracellularSpace, ExtracellularRegion);
    node!(membrane_enclosed_lumen: MembraneEnlosedLumen, CellularComponent);
    node!(organelle: Organelle, CellularComponent);
    node!(cytoskeleton: Cytoskeleton, NonMembraneOrganelle);
    node!(microtubule_cytoskeleton: MicrotubuleCytoskeleton, Cytoskeleton);
    node!(intracellular_organelle: IntracellularOrganelle, Organelle);
    node!(nucleus: Nucleus, MembraneOrganelle);
    node!(intracellular_organelle_lumen: IntracellularOrganelleLumen, OrganelleLumen);
    node!(extracellular_region_part: ExtracelularRegionPart, ExtracellularRegion, CellularComponent);
    node!(non_membrane_organelle: NonMembraneOrganelle, Organelle, IntracellularOrganelle);
    node!(organelle_part: OrganellePart, CellularComponent, Organelle);
    node!(intracellular_part: IntracellularPart, Intracellular, CellPart);
    node!(cytoskeletal_part: CytoskeletalPart, IntracellularOrganellePart, Cytoskeleton);
    node!(membrane_organelle: MembraneOrganelle, Organelle, IntracellularOrganelle);
    node!(nuclear_part: NuclearPart, Nucleus, IntracellularOrganellePart);
    node!(organelle_lumen: OrganelleLumen, OrganellePart, MembraneEnlosedLumen);
    node!(nuclear_lumen: NuclearLumen, NuclearPart, IntracellularOrganelleLumen);
    node!(intracellular_organelle_part: IntracellularOrganellePart, OrganellePart, IntracellularPart,  IntracellularOrganelle);
    node!(spindle: Spindle, MicrotubuleCytoskeleton, NonMembraneOrganelle, CytoskeletalPart);
}
