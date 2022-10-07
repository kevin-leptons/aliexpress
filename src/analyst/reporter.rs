use crate::analyst::model::{
    AnalysisModel, DatabaseSummaryModel, OrdersByPriceModel, ProductOrdersModel, ProductPriceModel,
    ProductRatingModel, ProspectProductModel, RevenueByPriceModel, RevenueByRatingModel,
    ShippingFeeModel, StoreNegativeRatingModel, StoreNeutralRatingModel, StoreOnlineModel,
    StorePositiveRatingModel, StoreRatingModel, TopCategoryModel, TopProductModel, TopStoreModel,
};
use crate::config::Config;
use crate::database::Database;
use crate::log::Log;
use crate::provider::Product;
use crate::result::BoxResult;
use chrono::{DateTime, Utc};
use fs_extra;
use fs_extra::dir::CopyOptions;
use hypermelon::elem::{Append, Elem};
use minify_html;
use poloto::num::timestamp::UnixTime;
use poloto::render::Theme;
use std::fs;
use std::path::PathBuf;
use tera::{Context, Tera};

pub struct Reporter<'a> {
    config: &'a Config,
    tera: Tera,
    svg_theme: Append<Theme<'a>, &'a str>,
}

impl<'a> Reporter<'a> {
    pub fn new(config: &'a Config) -> BoxResult<Self> {
        Self::prepare_report_directory(config)?;
        let template_directory = config.asset_directory.join("**/*.html");
        let tera = Tera::new(template_directory.to_str().unwrap())?;
        let instance = Self {
            config,
            tera,
            svg_theme: Self::initialize_svg_theme(),
        };
        return Ok(instance);
    }

    fn prepare_report_directory(config: &Config) -> BoxResult<()> {
        fs::create_dir_all(&config.report_directory)?;
        let sources = vec![
            config.asset_directory.join("css"),
            config.asset_directory.join("font"),
        ];
        let mut options = CopyOptions::new();
        options.overwrite = true;
        fs_extra::copy_items(&sources, &config.report_directory, &options)?;
        return Ok(());
    }

    fn initialize_svg_theme() -> Append<Theme<'a>, &'a str> {
        return poloto::render::Theme::light().append(
            r#"
            .poloto_background{fill-opacity:0;}
            .poloto0.poloto_histo.poloto_imgs{fill:#3B77BC;stroke:black;stroke-width:1px}
            .poloto_grid{display:none}
            .poloto0.poloto_stroke{stroke:#3B77BC}
        "#,
        );
    }

    pub fn make_index_report(&self, model: &AnalysisModel) -> BoxResult<PathBuf> {
        let output_path = self.config.report_directory.join("index.html");
        let context = Context::from_serialize(model)?;
        let output = self.tera.render("index.html", &context)?;
        write_minify_html(&output_path, &output)?;
        return Ok(output_path);
    }

    pub fn make_timeline_report(
        &self,
        store_online_model: &StoreOnlineModel,
    ) -> BoxResult<PathBuf> {
        let root_path = self.config.report_directory.clone();
        self.render_timeline_report_html(&root_path)?;
        self.render_timeline_store_online_svg(store_online_model, &root_path)?;
        return Ok(root_path);
    }

    fn render_timeline_report_html(&self, root_path: &PathBuf) -> BoxResult<()> {
        let output_path = root_path.join("timeline.html");
        let context = Context::default();
        let output = self.tera.render("timeline.html", &context)?;
        write_minify_html(&output_path, &output)?;
        return Ok(());
    }

    fn render_timeline_store_online_svg(
        &self,
        model: &StoreOnlineModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data: Vec<(UnixTime, f64)> = model
            .iter()
            .map(|point| {
                (
                    UnixTime::from(DateTime::<Utc>::from_utc(point.timestamp, Utc)),
                    point.count as f64,
                )
            })
            .collect();

        let plots = poloto::plots!(
            poloto::build::plot("").line().cloned(data.iter()),
            poloto::build::markers([], [0.0])
        );

        let svg_str = poloto::data(plots)
            .build_and_label(("Store Online Time", "Date", "Stores"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()?;
        let output_path = root_path.join("timeline_store_online.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn make_top_report(
        &self,
        products_by_rating: &TopProductModel,
        products_by_orders: &TopProductModel,
        products_by_revenue: &TopProductModel,
        products_by_highest_price: &TopProductModel,
        products_by_lowest_price: &TopProductModel,
        stores_by_revenue: &TopStoreModel,
        stores_by_orders: &TopStoreModel,
        stores_by_online_time: &TopStoreModel,
        level_1_categories_by_revenue: &TopCategoryModel,
        level_2_categories_by_revenue: &TopCategoryModel,
    ) -> BoxResult<PathBuf> {
        let root_path = &self.config.report_directory;
        fs::create_dir_all(&root_path)?;
        render_top_product_by_rating(&self.tera, products_by_rating, root_path)?;
        render_top_product_by_orders(&self.tera, products_by_orders, root_path)?;
        render_top_product_by_revenue(&self.tera, products_by_revenue, root_path)?;
        render_top_product_by_highest_price(&self.tera, products_by_highest_price, root_path)?;
        render_top_product_by_lowest_price(&self.tera, products_by_lowest_price, root_path)?;
        render_top_stores_by_revenue(&self.tera, stores_by_revenue, root_path)?;
        render_top_stores_by_orders(&self.tera, stores_by_orders, root_path)?;
        render_top_stores_by_online_time(&self.tera, stores_by_online_time, root_path)?;
        render_top_level_1_categories_by_revenue(
            &self.tera,
            level_1_categories_by_revenue,
            root_path,
        )?;
        render_top_level_2_categories_by_revenue(
            &self.tera,
            level_2_categories_by_revenue,
            root_path,
        )?;
        return Ok(root_path.clone());
    }

    pub fn make_distribution_report(
        &self,
        products_by_rating: &ProductRatingModel,
        products_by_orders: &ProductOrdersModel,
        products_by_price: &ProductPriceModel,
        products_by_shipping_fee: &ShippingFeeModel,
        stores_by_rating: &StoreRatingModel,
        stores_by_rating45_count: &StorePositiveRatingModel,
        stores_by_rating12_count: &StoreNegativeRatingModel,
        stores_by_rating3_count: &StoreNeutralRatingModel,
        revenue_by_rating: &RevenueByRatingModel,
        revenue_by_price: &RevenueByPriceModel,
        orders_by_price: &OrdersByPriceModel,
    ) -> BoxResult<PathBuf> {
        let root_path = self.config.report_directory.clone();
        self.render_distribution_html(&root_path)?;
        self.render_distribution_products_by_rating_svg(products_by_rating, &root_path)?;
        self.render_distribution_products_by_orders_svg(products_by_orders, &root_path)?;
        self.render_distribution_products_by_price_svg(products_by_price, &root_path)?;
        self.render_distribution_products_by_shipping_fee_svg(
            products_by_shipping_fee,
            &root_path,
        )?;
        self.render_distribution_stores_by_rating_svg(stores_by_rating, &root_path)?;
        self.render_distribution_stores_by_rating45_count_svg(
            stores_by_rating45_count,
            &root_path,
        )?;
        self.render_distribution_stores_by_rating12_count_svg(
            stores_by_rating12_count,
            &root_path,
        )?;
        self.render_distribution_stores_by_rating3_count_svg(stores_by_rating3_count, &root_path)?;
        self.render_distribution_revenue_by_rating_svg(revenue_by_rating, &root_path)?;
        self.render_distribution_revenue_by_price_svg(revenue_by_price, &root_path)?;
        self.render_distribution_orders_by_price_svg(orders_by_price, &root_path)?;
        return Ok(root_path);
    }

    fn render_distribution_html(&self, root_path: &PathBuf) -> BoxResult<()> {
        let output_path = root_path.join("distribution.html");
        let context = Context::default();
        let output = self.tera.render("distribution.html", &context)?;
        write_minify_html(&output_path, &output)?;
        return Ok(());
    }

    fn render_distribution_products_by_rating_svg(
        &self,
        model: &ProductRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_1_2 as i128, "[1, 2)"),
            (model.count_2_3 as i128, "[2, 3)"),
            (model.count_3_4 as i128, "[3, 4)"),
            (model.count_4_5 as i128, "[4, 5)"),
            (model.count_5 as i128, "5"),
            (model.unknown_count as i128, "unknown"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Quantity of Product by Rating", "Products", "Rating"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_products_by_rating.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    fn render_distribution_products_by_orders_svg(
        &self,
        model: &ProductOrdersModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0 as i128, "0"),
            (model.count_1_9 as i128, "1-9"),
            (model.count_10_19 as i128, "10-19"),
            (model.count_20_49 as i128, "20-49"),
            (model.count_50_99 as i128, "50-99"),
            (model.count_100_499 as i128, "100-499"),
            (model.count_500_999 as i128, "500-999"),
            (model.count_1000_9999 as i128, "1000-9999"),
            (model.count_10000_n as i128, ">= 10000"),
            (model.unknown_count as i128, "unknown"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Quantity Product by Orders", "Products", "Orders"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_products_by_orders.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    fn render_distribution_products_by_price_svg(
        &self,
        model: &ProductPriceModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0_1 as i128, "[0-1)"),
            (model.count_1_5 as i128, "[1-5)"),
            (model.count_5_10 as i128, "[5-10)"),
            (model.count_10_20 as i128, "[10-20)"),
            (model.count_20_30 as i128, "[20-30)"),
            (model.count_30_50 as i128, "[30-50)"),
            (model.count_50_100 as i128, "[50-100)"),
            (model.count_100_1000 as i128, "[100-1000)"),
            (model.count_1000_n as i128, ">= 1000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Products by Price", "Products", "Price"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_products_by_price.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    fn render_distribution_products_by_shipping_fee_svg(
        &self,
        model: &ShippingFeeModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0 as i128, "0"),
            (model.count_0_1 as i128, "(0, 1]"),
            (model.count_1_5 as i128, "(1, 5]"),
            (model.count_5_10 as i128, "(5, 10]"),
            (model.count_10_20 as i128, "(10, 20]"),
            (model.count_20_30 as i128, "(20, 30]"),
            (model.count_30_50 as i128, "(30, 50]"),
            (model.count_50_100 as i128, "(50,100]"),
            (model.count_100_n as i128, "over 100"),
            (model.unknown_count as i128, "unknown"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Products by Shipping Fee", "Products", "Shipping Fee"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_products_by_shipping_fee.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    fn render_distribution_stores_by_rating_svg(
        &self,
        model: &StoreRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0_10 as i128, "[0, 10)"),
            (model.count_10_30 as i128, "[10, 30)"),
            (model.count_30_50 as i128, "[30, 50)"),
            (model.count_50_80 as i128, "[50, 80)"),
            (model.count_80_90 as i128, "[80, 90)"),
            (model.count_90_95 as i128, "[90, 95)"),
            (model.count_95_96 as i128, "[95, 96)"),
            (model.count_96_97 as i128, "[96, 97)"),
            (model.count_97_98 as i128, "[97, 98)"),
            (model.count_98_99 as i128, "[98, 99)"),
            (model.count_99_100 as i128, "[99, 100]"),
            (model.unknown_count as i128, "unknown"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Stores by Rating", "Stores", "Positive Rating %"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_stores_by_rating.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_stores_by_rating45_count_svg(
        &self,
        model: &StorePositiveRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0_10 as i128, "[0, 10)"),
            (model.count_10_20 as i128, "[10, 20)"),
            (model.count_20_30 as i128, "[20, 30)"),
            (model.count_30_50 as i128, "[30, 50)"),
            (model.count_50_100 as i128, "[50, 100)"),
            (model.count_100_500 as i128, "[100, 500)"),
            (model.count_500_1000 as i128, "[500, 1000)"),
            (model.count_1000_2000 as i128, "[1000, 2000)"),
            (model.count_2000_3000 as i128, "[2000, 3000)"),
            (model.count_3000_5000 as i128, "[3000, 5000)"),
            (model.count_5000_n as i128, ">= 5000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label((
                "Stores by Positive Rating",
                "Stores",
                "Quantity of Positive Rating",
            ))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_stores_by_rating45_count.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_stores_by_rating12_count_svg(
        &self,
        model: &StoreNegativeRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0_10 as i128, "[0, 10)"),
            (model.count_10_20 as i128, "[10, 20)"),
            (model.count_20_30 as i128, "[20, 30)"),
            (model.count_30_50 as i128, "[30, 50)"),
            (model.count_50_100 as i128, "[50, 100)"),
            (model.count_100_500 as i128, "[100, 500)"),
            (model.count_500_1000 as i128, "[500, 1000)"),
            (model.count_1000_2000 as i128, "[1000, 2000)"),
            (model.count_2000_3000 as i128, "[2000, 3000)"),
            (model.count_3000_5000 as i128, "[3000, 5000)"),
            (model.count_5000_n as i128, ">= 5000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label((
                "Stores by Negative Rating",
                "Stores",
                "Quantity of Negative Rating",
            ))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_stores_by_rating12_count.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_stores_by_rating3_count_svg(
        &self,
        model: &StoreNeutralRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.count_0_10 as i128, "[0, 10)"),
            (model.count_10_20 as i128, "[10, 20)"),
            (model.count_20_30 as i128, "[20, 30)"),
            (model.count_30_50 as i128, "[30, 50)"),
            (model.count_50_100 as i128, "[50, 100)"),
            (model.count_100_500 as i128, "[100, 500)"),
            (model.count_500_1000 as i128, "[500, 1000)"),
            (model.count_1000_2000 as i128, "[1000, 2000)"),
            (model.count_2000_3000 as i128, "[2000, 3000)"),
            (model.count_3000_5000 as i128, "[3000, 5000)"),
            (model.count_5000_n as i128, ">= 5000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Stores by Neutral", "Stores", "Quantity of Neutral Rating"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_stores_by_rating3_count.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_revenue_by_rating_svg(
        &self,
        model: &RevenueByRatingModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.rating_1_2, "[1, 2)"),
            (model.rating_2_3, "[2, 3)"),
            (model.rating_3_4, "[3, 4)"),
            (model.rating_4_5, "[4, 5)"),
            (model.rating_5, "5"),
            (model.unknown, "unknown"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0.0])
            .label(("Revenue by Rating", "Revenue", "Rating"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_revenue_by_rating.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_revenue_by_price_svg(
        &self,
        model: &RevenueByPriceModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.price_0_1, "[0, 1)"),
            (model.price_1_5, "[1, 5)"),
            (model.price_5_10, "[5, 10)"),
            (model.price_10_20, "[10, 20)"),
            (model.price_20_30, "[20, 30)"),
            (model.price_30_50, "[30, 50)"),
            (model.price_50_100, "[50, 100)"),
            (model.price_100_200, "[100, 200)"),
            (model.price_200_500, "[200, 500)"),
            (model.price_500_700, "[500, 700)"),
            (model.price_700_1000, "[700, 1000)"),
            (model.price_1000_n, ">= 1000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0.0])
            .label(("Revenue by Price", "Revenue", "Price"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_revenue_by_price.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn render_distribution_orders_by_price_svg(
        &self,
        model: &OrdersByPriceModel,
        root_path: &PathBuf,
    ) -> BoxResult<()> {
        let data = [
            (model.price_0_1 as i128, "[0, 1)"),
            (model.price_1_5 as i128, "[1, 5)"),
            (model.price_5_10 as i128, "[5, 10)"),
            (model.price_10_20 as i128, "[10, 20)"),
            (model.price_20_30 as i128, "[20, 30)"),
            (model.price_30_50 as i128, "[30, 50)"),
            (model.price_50_100 as i128, "[50, 100)"),
            (model.price_100_200 as i128, "[100, 200)"),
            (model.price_200_500 as i128, "[200, 500)"),
            (model.price_500_700 as i128, "[500, 700)"),
            (model.price_700_1000 as i128, "[700, 1000)"),
            (model.price_1000_n as i128, ">= 1000"),
        ];
        let svg_str = poloto::build::bar::gen_simple("", data, [0])
            .label(("Orders by Price", "Orders", "Price"))
            .append_to(poloto::header().append(self.svg_theme))
            .render_string()
            .unwrap();
        let output_path = root_path.join("distribution_orders_by_price.svg");
        std::fs::write(output_path, svg_str)?;
        return Ok(());
    }

    pub fn make_prospect_reports(
        &self,
        products_by_points: &ProspectProductModel,
    ) -> BoxResult<PathBuf> {
        let root_path = self.config.report_directory.clone();
        render_prospect_products_by_points(&self.tera, products_by_points, &root_path)?;
        return Ok(root_path);
    }
}

fn render_top_product_by_rating(
    tera: &Tera,
    model: &TopProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_products_by_rating.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_product_by_orders(
    tera: &Tera,
    model: &TopProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_products_by_orders.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_product_by_revenue(
    tera: &Tera,
    model: &TopProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_products_by_revenue.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_product_by_highest_price(
    tera: &Tera,
    model: &TopProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_products_by_highest_price.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_product_by_lowest_price(
    tera: &Tera,
    model: &TopProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_products_by_lowest_price.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_stores_by_revenue(
    tera: &Tera,
    model: &TopStoreModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_stores_by_revenue.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_stores_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_stores_by_orders(
    tera: &Tera,
    model: &TopStoreModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_stores_by_orders.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_stores_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_stores_by_online_time(
    tera: &Tera,
    model: &TopStoreModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_stores_by_online_time.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_stores_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_level_1_categories_by_revenue(
    tera: &Tera,
    model: &TopCategoryModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_level_1_categories_by_revenue.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_categories_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_top_level_2_categories_by_revenue(
    tera: &Tera,
    model: &TopCategoryModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("top_level_2_categories_by_revenue.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_top_categories_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn render_prospect_products_by_points(
    tera: &Tera,
    model: &ProspectProductModel,
    root_path: &PathBuf,
) -> BoxResult<()> {
    let output_path = root_path.join("prospect_products_by_points.html");
    let context = Context::from_serialize(model)?;
    let output = tera.render("_prospect_products_by.html", &context)?;
    write_minify_html(&output_path, &output)?;
    return Ok(());
}

fn write_minify_html(target: &PathBuf, html_str: &String) -> BoxResult<()> {
    let mut cfg = minify_html::Cfg::new();
    let minified_html_str = minify_html::minify(html_str.as_bytes(), &cfg);
    fs::write(&target, minified_html_str)?;
    return Ok(());
}
