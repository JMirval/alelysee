use anyhow::Result;
use headless_chrome::{Browser as ChromeBrowser, LaunchOptions, Tab};
use std::sync::Arc;

pub struct Browser {
    browser: ChromeBrowser,
}

impl Browser {
    pub fn launch() -> Result<Self> {
        let options = LaunchOptions::default_builder()
            .headless(true)
            .build()
            .expect("Failed to build launch options");

        let browser = ChromeBrowser::new(options)?;

        Ok(Self { browser })
    }

    pub fn new_page(&self) -> Result<Page> {
        let tab = self.browser.new_tab()?;
        Ok(Page { tab })
    }
}

pub struct Page {
    tab: Arc<Tab>,
}

impl Page {
    pub fn goto(&self, url: &str) -> Result<()> {
        self.tab.navigate_to(url)?;
        self.tab.wait_until_navigated()?;
        Ok(())
    }

    pub fn find_element(&self, selector: &str) -> Result<String> {
        let element = self.tab.wait_for_element(selector)?;
        let text = element.get_inner_text()?;
        Ok(text)
    }

    pub fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        let element = self.tab.wait_for_element(selector)?;
        element.click()?;
        element.type_into(text)?;
        Ok(())
    }

    pub fn click(&self, selector: &str) -> Result<()> {
        let element = self.tab.wait_for_element(selector)?;
        element.click()?;
        Ok(())
    }

    pub fn url(&self) -> Result<String> {
        Ok(self.tab.get_url())
    }
}
