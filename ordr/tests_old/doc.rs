use ordr::{build, job::Job, producer};

macro_rules! node {
    // No deps
    ( $name:ident: $ty:ident ) => {
        #[derive(Debug, Clone)]
        struct $ty;

        #[producer]
        async fn $name(_ctx: ()) -> Result<$ty, std::convert::Infallible> {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok($ty)
        }
    };
    // Deps
    ( $name:ident: $ty:ident, $( $dep:ident ),* ) => {
        #[derive(Debug, Clone)]
        struct $ty;

        #[producer]
        async fn $name(_ctx: (), $( _: $dep ),* ) -> Result<$ty, std::convert::Infallible> {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok($ty)
        }
    };
}

node!(s3_prefix_stored: S3PrefixStored);
node!(original_file_path: OriginalFilePath);
node!(file_meta: FileMeta, OriginalFilePath);
node!(doc: Doc, OriginalFilePath, FileMeta);
node!(doc_uploaded: DocUploaded, Doc);
node!(doc_structure: DocStructure, Doc);
node!(doc_structure_uploaded: DocStructureUploaded, DocStructure);
node!(text_info: TextInfo, Doc);
node!(text_info_uploaded: TextInfoUploaded, TextInfo);
node!(text_info_stored: TextInfoStored, TextInfo, TextInfoUploaded);
node!(doc_meta: DocMeta, Doc, FileMeta);
node!(doc_meta_stored: DocMetaStored, DocMeta, DocCoverUploaded);
node!(doc_cover: DocCover, Doc);
node!(doc_cover_uploaded: DocCoverUploaded, DocCover);
node!(links: Links, DocStructure, Doc);
node!(links_stored: LinksStored, Links);
node!(doc_text: DocText, Doc, DocMeta);
node!(doc_text_uploaded: DocTextUploaded, DocText);
node!(layers: Layers, DocMeta, Doc);
node!(layers_uploaded: LayersUploaded, Layers);
node!(layers_meta_stored: LayersMetaStored, Layers, LayersUploaded);
node!(language: Language, DocText);
node!(language_stored: LanguageStored, Language);
node!(doc_page_images: DocPageImages, Doc, DocMeta, Layers);
node!(doc_page_images_uploaded: DocPageImagesUploaded, DocPageImages);
node!(lda_meta: LdaMeta, Language, DocText);
node!(lda_meta_stored: LdaMetaStored, LdaMeta);
node!(pages_jsonp: PagesJsonp, DocText, DocMeta, DocPageImages);
node!(pages_jsonp_uploaded: PagesJsonpUploaded, PagesJsonp);
node!(tmp_file_copied: TmpFileCopied, Doc);
node!(spam_notified: SpamNotified, DocMeta);
node!(categories_detected: CategoriesDetected, DocText);
node!(structed_data: StructuredData, DocStructureUploaded, DocPageImagesUploaded);

#[tokio::test]
async fn doc_example() {
    // tracing_subscriber::fmt().init();

    let graph = build!(
        S3PrefixStored,
        OriginalFilePath,
        FileMeta,
        Doc,
        DocUploaded,
        DocStructure,
        DocStructureUploaded,
        TextInfo,
        TextInfoUploaded,
        TextInfoStored,
        DocMeta,
        DocMetaStored,
        DocCover,
        DocCoverUploaded,
        Links,
        LinksStored,
        DocText,
        DocTextUploaded,
        Layers,
        LayersUploaded,
        LayersMetaStored,
        Language,
        LanguageStored,
        DocPageImages,
        DocPageImagesUploaded,
        LdaMeta,
        LdaMetaStored,
        PagesJsonp,
        PagesJsonpUploaded,
        TmpFileCopied,
        SpamNotified,
        CategoriesDetected,
        StructuredData,
    )
    .unwrap();

    let job = Job::new()
        .with_target::<S3PrefixStored>()
        .with_target::<TextInfoStored>()
        .with_target::<LanguageStored>()
        .with_target::<LdaMetaStored>()
        .with_target::<LinksStored>()
        .with_target::<DocUploaded>()
        .with_target::<PagesJsonpUploaded>()
        .with_target::<CategoriesDetected>()
        .with_target::<DocTextUploaded>()
        .with_target::<DocMetaStored>()
        .with_target::<LayersMetaStored>()
        .with_target::<StructuredData>()
        .with_target::<TmpFileCopied>()
        .with_target::<SpamNotified>();

    // let diagram = graph.mermaid(&job);
    // println!("{diagram}");

    graph.execute(job, ()).await.unwrap();
}
