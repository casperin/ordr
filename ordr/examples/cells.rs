use ordr::{build, job::Job};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let graph = build!(
        cells::CellularComponent,
        cells::Cell,
        cells::CellPart,
        cells::CellSurface,
        cells::Intracellular,
        cells::ExtracellularRegion,
        cells::ExtracelularRegionPart,
        cells::ExtracellularSpace,
        cells::MembraneEnlosedLumen,
        cells::Organelle,
        cells::NonMembraneOrganelle,
        cells::Cytoskeleton,
        cells::MicrotubuleCytoskeleton,
        cells::IntracellularOrganelle,
        cells::OrganellePart,
        cells::IntracellularPart,
        cells::IntracellularOrganellePart,
        cells::CytoskeletalPart,
        cells::Spindle,
        cells::MembraneOrganelle,
        cells::Nucleus,
        cells::NuclearPart,
        cells::OrganelleLumen,
        cells::IntracellularOrganelleLumen,
        cells::NuclearLumen,
    )
    .unwrap();

    let job = Job::new()
        // .with_input(cells::IntracellularPart)
        .with_target::<cells::Nucleus>()
        .with_target::<cells::Spindle>();

    let diagram = graph.mermaid(&job);
    println!("{diagram}");

    let ctx = cells::Ctx::new();

    let res = graph.execute(job, ctx).await;
    let _ = dbg!(res);
}

mod cells {
    use std::{convert::Infallible, sync::Arc, time::Duration};

    use ordr::producer;
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

    #[derive(Clone, Debug)]
    pub struct CellularComponent;

    #[producer]
    async fn cellular_component(ctx: Ctx) -> Result<CellularComponent, Infallible> {
        ctx.wait().await;
        Ok(CellularComponent)
    }

    #[derive(Clone, Debug)]
    pub struct Cell;

    #[producer]
    async fn cell(ctx: Ctx, _: CellularComponent) -> Result<Cell, Infallible> {
        ctx.wait().await;
        Ok(Cell)
    }

    #[derive(Clone, Debug)]
    pub struct CellPart;

    #[producer]
    async fn cell_part(ctx: Ctx, _: Cell) -> Result<CellPart, Infallible> {
        ctx.wait().await;
        Ok(CellPart)
    }

    #[derive(Clone, Debug)]
    pub struct CellSurface;

    #[producer]
    async fn cell_surface(ctx: Ctx, _: CellPart) -> Result<CellSurface, Infallible> {
        ctx.wait().await;
        Ok(CellSurface)
    }

    #[derive(Clone, Debug)]
    pub struct Intracellular;

    #[producer]
    async fn intracellular(ctx: Ctx, _: CellPart) -> Result<Intracellular, Infallible> {
        ctx.wait().await;
        Ok(Intracellular)
    }

    #[derive(Clone, Debug)]
    pub struct ExtracellularRegion;

    #[producer]
    async fn extracellular_region(
        ctx: Ctx,
        _: CellularComponent,
    ) -> Result<ExtracellularRegion, Infallible> {
        ctx.wait().await;
        Ok(ExtracellularRegion)
    }

    #[derive(Clone, Debug)]
    pub struct ExtracellularSpace;

    #[producer]
    async fn extracellular_space(
        ctx: Ctx,
        _: ExtracellularRegion,
    ) -> Result<ExtracellularSpace, Infallible> {
        ctx.wait().await;
        Ok(ExtracellularSpace)
    }

    #[derive(Clone, Debug)]
    pub struct MembraneEnlosedLumen;

    #[producer]
    async fn membrane_enclosed_lumen(
        ctx: Ctx,
        _: CellularComponent,
    ) -> Result<MembraneEnlosedLumen, Infallible> {
        ctx.wait().await;
        Ok(MembraneEnlosedLumen)
    }

    #[derive(Clone, Debug)]
    pub struct Organelle;

    #[producer]
    async fn organelle(ctx: Ctx, _: CellularComponent) -> Result<Organelle, Infallible> {
        ctx.wait().await;
        Ok(Organelle)
    }

    #[derive(Clone, Debug)]
    pub struct Cytoskeleton;

    #[producer]
    async fn cytoskeleton(ctx: Ctx, _: NonMembraneOrganelle) -> Result<Cytoskeleton, Infallible> {
        ctx.wait().await;
        Ok(Cytoskeleton)
    }

    #[derive(Clone, Debug)]
    pub struct MicrotubuleCytoskeleton;

    #[producer]
    async fn microtubule_cytoskeleton(
        ctx: Ctx,
        _: Cytoskeleton,
    ) -> Result<MicrotubuleCytoskeleton, Infallible> {
        ctx.wait().await;
        Ok(MicrotubuleCytoskeleton)
    }

    #[derive(Clone, Debug)]
    pub struct IntracellularOrganelle;

    #[producer]
    async fn intracellular_organelle(
        ctx: Ctx,
        _: Organelle,
    ) -> Result<IntracellularOrganelle, Infallible> {
        ctx.wait().await;
        Ok(IntracellularOrganelle)
    }

    #[derive(Clone, Debug)]
    pub struct Nucleus;

    #[producer]
    async fn nucleus(ctx: Ctx, _: MembraneOrganelle) -> Result<Nucleus, Infallible> {
        ctx.wait().await;
        Ok(Nucleus)
    }

    #[derive(Clone, Debug)]
    pub struct IntracellularOrganelleLumen;

    #[producer]
    async fn intracellular_organelle_lumen(
        ctx: Ctx,
        _: OrganelleLumen,
    ) -> Result<IntracellularOrganelleLumen, Infallible> {
        ctx.wait().await;
        Ok(IntracellularOrganelleLumen)
    }

    #[derive(Clone, Debug)]
    pub struct ExtracelularRegionPart;

    #[producer]
    async fn extracellular_region_part(
        ctx: Ctx,
        _: ExtracellularRegion,
        _: CellularComponent,
    ) -> Result<ExtracelularRegionPart, Infallible> {
        ctx.wait().await;
        Ok(ExtracelularRegionPart)
    }

    #[derive(Clone, Debug)]
    pub struct NonMembraneOrganelle;

    #[producer]
    async fn non_membrane_organelle(
        ctx: Ctx,
        _: Organelle,
        _: IntracellularOrganelle,
    ) -> Result<NonMembraneOrganelle, Infallible> {
        ctx.wait().await;
        Ok(NonMembraneOrganelle)
    }

    #[derive(Clone, Debug)]
    pub struct OrganellePart;

    #[producer]
    async fn organelle_part(
        ctx: Ctx,
        _: CellularComponent,
        _: Organelle,
    ) -> Result<OrganellePart, Infallible> {
        ctx.wait().await;
        Ok(OrganellePart)
    }

    #[derive(Clone, Debug)]
    pub struct IntracellularPart;

    #[producer]
    async fn intracellular_part(
        ctx: Ctx,
        _: Intracellular,
        _: CellPart,
    ) -> Result<IntracellularPart, Infallible> {
        ctx.wait().await;
        Ok(IntracellularPart)
    }

    #[derive(Clone, Debug)]
    pub struct CytoskeletalPart;

    #[producer]
    async fn cytoskeletal_part(
        ctx: Ctx,
        _: IntracellularOrganellePart,
        _: Cytoskeleton,
    ) -> Result<CytoskeletalPart, Infallible> {
        ctx.wait().await;
        Ok(CytoskeletalPart)
    }

    #[derive(Clone, Debug)]
    pub struct MembraneOrganelle;

    #[producer]
    async fn membrane_organelle(
        ctx: Ctx,
        _: Organelle,
        _: IntracellularOrganelle,
    ) -> Result<MembraneOrganelle, Infallible> {
        ctx.wait().await;
        Ok(MembraneOrganelle)
    }

    #[derive(Clone, Debug)]
    pub struct NuclearPart;

    #[producer]
    async fn nuclear_part(
        ctx: Ctx,
        _: Nucleus,
        _: IntracellularOrganellePart,
    ) -> Result<NuclearPart, Infallible> {
        ctx.wait().await;
        Ok(NuclearPart)
    }

    #[derive(Clone, Debug)]
    pub struct OrganelleLumen;

    #[producer]
    async fn organelle_lumen(
        ctx: Ctx,
        _: OrganellePart,
        _: MembraneEnlosedLumen,
    ) -> Result<OrganelleLumen, Infallible> {
        ctx.wait().await;
        Ok(OrganelleLumen)
    }

    #[derive(Clone, Debug)]
    pub struct NuclearLumen;

    #[producer]
    async fn nuclear_lumen(
        ctx: Ctx,
        _: NuclearPart,
        _: IntracellularOrganelleLumen,
    ) -> Result<NuclearLumen, Infallible> {
        ctx.wait().await;
        Ok(NuclearLumen)
    }

    #[derive(Clone, Debug)]
    pub struct IntracellularOrganellePart;
    #[producer]
    async fn intracellular_organelle_part(
        ctx: Ctx,
        _: OrganellePart,
        _: IntracellularPart,
        _: IntracellularOrganelle,
    ) -> Result<IntracellularOrganellePart, Infallible> {
        ctx.wait().await;
        Ok(IntracellularOrganellePart)
    }

    #[derive(Clone, Debug)]
    pub struct Spindle;

    #[producer]
    async fn spindle(
        ctx: Ctx,
        _: MicrotubuleCytoskeleton,
        _: NonMembraneOrganelle,
        _: CytoskeletalPart,
    ) -> Result<Spindle, Infallible> {
        ctx.wait().await;
        Ok(Spindle)
    }
}
