use color_eyre::eyre::Result;
use rand::Rng;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::ui::UI;

// weather stations with their mean temperatures (from original 1brc)
const STATIONS: &[(&str, f64)] = &[
    ("Abha", 18.0),
    ("Abidjan", 26.0),
    ("Abéché", 29.4),
    ("Accra", 26.4),
    ("Addis Ababa", 16.0),
    ("Adelaide", 17.3),
    ("Aden", 29.1),
    ("Ahvaz", 25.4),
    ("Albuquerque", 14.0),
    ("Alexandra", 11.0),
    ("Alexandria", 20.0),
    ("Algiers", 18.2),
    ("Alice Springs", 21.0),
    ("Almaty", 10.0),
    ("Amsterdam", 10.2),
    ("Anadyr", -6.9),
    ("Anchorage", 2.8),
    ("Andorra la Vella", 9.8),
    ("Ankara", 12.0),
    ("Antananarivo", 17.9),
    ("Antsiranana", 25.2),
    ("Arkhangelsk", 1.3),
    ("Ashgabat", 17.1),
    ("Asmara", 15.6),
    ("Assab", 30.5),
    ("Astana", 3.5),
    ("Athens", 19.2),
    ("Atlanta", 17.0),
    ("Auckland", 15.2),
    ("Austin", 20.7),
    ("Baghdad", 22.8),
    ("Baguio", 19.5),
    ("Baku", 15.1),
    ("Baltimore", 13.1),
    ("Bamako", 27.8),
    ("Bangkok", 28.6),
    ("Bangui", 26.0),
    ("Banjul", 26.0),
    ("Barcelona", 18.2),
    ("Bata", 25.1),
    ("Batumi", 14.0),
    ("Beijing", 12.9),
    ("Beirut", 20.9),
    ("Belgrade", 12.5),
    ("Belize City", 26.7),
    ("Benghazi", 19.9),
    ("Bergen", 7.7),
    ("Berlin", 10.3),
    ("Bilbao", 14.7),
    ("Birao", 26.5),
    ("Bishkek", 11.3),
    ("Bissau", 27.0),
    ("Blantyre", 22.2),
    ("Bloemfontein", 15.6),
    ("Boise", 11.4),
    ("Bordeaux", 14.2),
    ("Bosaso", 30.0),
    ("Boston", 10.9),
    ("Bouaké", 26.0),
    ("Bratislava", 10.5),
    ("Brazzaville", 25.0),
    ("Bridgetown", 27.0),
    ("Brisbane", 21.4),
    ("Brussels", 10.5),
    ("Bucharest", 10.8),
    ("Budapest", 11.3),
    ("Bujumbura", 23.8),
    ("Bulawayo", 18.9),
    ("Burnie", 13.0),
    ("Busan", 15.0),
    ("Cabo San Lucas", 23.9),
    ("Cairns", 25.0),
    ("Cairo", 21.4),
    ("Calgary", 4.4),
    ("Canberra", 13.1),
    ("Cape Town", 16.2),
    ("Casablanca", 17.7),
    ("Cayenne", 27.0),
    ("Charlotte", 16.1),
    ("Chiang Mai", 25.8),
    ("Chicago", 9.8),
    ("Chihuahua", 18.6),
    ("Chișinău", 10.2),
    ("Chittagong", 25.9),
    ("Chongqing", 18.6),
    ("Christchurch", 12.2),
    ("City of San Marino", 11.8),
    ("Colombo", 27.4),
    ("Columbus", 11.7),
    ("Conakry", 26.4),
    ("Copenhagen", 9.1),
    ("Cotonou", 27.2),
    ("Cracow", 9.3),
    ("Da Lat", 17.9),
    ("Da Nang", 25.8),
    ("Dakar", 24.0),
    ("Dallas", 19.0),
    ("Damascus", 17.0),
    ("Dampier", 26.4),
    ("Dar es Salaam", 25.8),
    ("Darwin", 27.6),
    ("Denpasar", 23.7),
    ("Denver", 10.4),
    ("Detroit", 10.0),
    ("Dhaka", 25.9),
    ("Dikson", -11.1),
    ("Dili", 26.6),
    ("Djibouti", 29.9),
    ("Dodoma", 22.7),
    ("Dolisie", 24.0),
    ("Douala", 26.7),
    ("Dubai", 26.9),
    ("Dublin", 9.8),
    ("Dunedin", 11.1),
    ("Durban", 20.6),
    ("Dushanbe", 14.7),
    ("Edinburgh", 9.3),
    ("Edmonton", 4.2),
    ("El Paso", 18.1),
    ("Entebbe", 21.0),
    ("Erbil", 19.5),
    ("Erzurum", 5.1),
    ("Fairbanks", -2.3),
    ("Fianarantsoa", 17.9),
    ("Flores, Petén", 26.4),
    ("Frankfurt", 10.6),
    ("Freetown", 26.8),
    ("Fresno", 17.9),
    ("Fukuoka", 17.0),
    ("Gaborone", 21.0),
    ("Gabès", 19.5),
    ("Gangtok", 15.2),
    ("Garissa", 29.3),
    ("Garoua", 28.3),
    ("George Town", 27.9),
    ("Ghanzi", 21.4),
    ("Gjoa Haven", -14.4),
    ("Guadalajara", 20.9),
    ("Guangzhou", 22.4),
    ("Guatemala City", 20.4),
    ("Halifax", 7.5),
    ("Hamburg", 9.7),
    ("Hamilton", 13.8),
    ("Hanga Roa", 20.5),
    ("Hanoi", 23.6),
    ("Harare", 18.4),
    ("Harbin", 5.0),
    ("Hargeisa", 21.7),
    ("Hat Yai", 27.0),
    ("Havana", 25.2),
    ("Helsinki", 5.9),
    ("Heraklion", 18.9),
    ("Hiroshima", 16.3),
    ("Ho Chi Minh City", 27.4),
    ("Hobart", 12.7),
    ("Hong Kong", 23.3),
    ("Honiara", 26.5),
    ("Honolulu", 25.4),
    ("Houston", 20.8),
    ("Ifrane", 11.4),
    ("Indianapolis", 11.8),
    ("Iqaluit", -9.3),
    ("Irkutsk", 1.0),
    ("Istanbul", 13.7),
    ("Jacksonville", 20.3),
    ("Jakarta", 26.7),
    ("Jayapura", 27.0),
    ("Jerusalem", 18.3),
    ("Johannesburg", 15.5),
    ("Jos", 22.8),
    ("Juba", 27.8),
    ("Kabul", 12.1),
    ("Kampala", 20.0),
    ("Kandi", 27.7),
    ("Kankan", 26.5),
    ("Kano", 26.4),
    ("Kansas City", 12.5),
    ("Karachi", 26.0),
    ("Karonga", 24.4),
    ("Kathmandu", 18.3),
    ("Khartoum", 29.9),
    ("Kingston", 27.4),
    ("Kinshasa", 25.3),
    ("Kolkata", 26.7),
    ("Kuala Lumpur", 27.3),
    ("Kumasi", 26.0),
    ("Kunming", 15.7),
    ("Kuopio", 3.4),
    ("Kuwait City", 25.7),
    ("Kyiv", 8.4),
    ("Kyoto", 15.8),
    ("La Ceiba", 26.2),
    ("La Paz", 23.7),
    ("Lagos", 26.8),
    ("Lahore", 24.3),
    ("Lake Havasu City", 23.7),
    ("Lake Tekapo", 8.7),
    ("Las Palmas de Gran Canaria", 21.2),
    ("Las Vegas", 20.3),
    ("Launceston", 13.1),
    ("Lhasa", 7.6),
    ("Libreville", 25.9),
    ("Lisbon", 17.5),
    ("Livingstone", 21.8),
    ("Ljubljana", 10.9),
    ("Lodwar", 29.3),
    ("Lomé", 26.9),
    ("London", 11.3),
    ("Los Angeles", 18.6),
    ("Louisville", 13.9),
    ("Luanda", 25.8),
    ("Lubumbashi", 20.8),
    ("Lusaka", 19.9),
    ("Luxembourg City", 9.3),
    ("Lviv", 7.8),
    ("Lyon", 12.5),
    ("Madrid", 15.0),
    ("Mahajanga", 26.3),
    ("Makassar", 26.7),
    ("Makurdi", 26.0),
    ("Malabo", 26.3),
    ("Malé", 28.0),
    ("Managua", 27.3),
    ("Manama", 26.5),
    ("Mandalay", 28.0),
    ("Mango", 28.1),
    ("Manila", 28.4),
    ("Maputo", 22.8),
    ("Marrakesh", 19.6),
    ("Marseille", 15.8),
    ("Maun", 22.4),
    ("Medan", 26.5),
    ("Mek'ele", 22.7),
    ("Melbourne", 15.1),
    ("Memphis", 17.2),
    ("Mexicali", 23.1),
    ("Mexico City", 17.5),
    ("Miami", 24.9),
    ("Milan", 13.0),
    ("Milwaukee", 8.9),
    ("Minneapolis", 7.8),
    ("Minsk", 6.7),
    ("Mogadishu", 27.1),
    ("Mombasa", 26.3),
    ("Monaco", 16.4),
    ("Moncton", 6.1),
    ("Monterrey", 22.3),
    ("Montreal", 6.8),
    ("Moscow", 5.8),
    ("Mumbai", 27.1),
    ("Murmansk", 0.6),
    ("Muscat", 28.0),
    ("Mzuzu", 17.7),
    ("N'Djamena", 28.3),
    ("Naha", 23.1),
    ("Nairobi", 17.8),
    ("Nakhon Ratchasima", 27.3),
    ("Napier", 14.6),
    ("Napoli", 15.9),
    ("Nashville", 15.4),
    ("Nassau", 25.0),
    ("Ndola", 20.3),
    ("New Delhi", 25.0),
    ("New Orleans", 20.7),
    ("New York City", 12.9),
    ("Ngaoundéré", 22.0),
    ("Niamey", 29.3),
    ("Nicosia", 19.7),
    ("Nouadhibou", 21.3),
    ("Nouakchott", 25.7),
    ("Novosibirsk", 1.7),
    ("Nuuk", -1.4),
    ("Odesa", 10.7),
    ("Okayama", 16.2),
    ("Okinawa", 23.1),
    ("Oklahoma City", 15.9),
    ("Omaha", 10.6),
    ("Oranjestad", 28.1),
    ("Oslo", 5.7),
    ("Ottawa", 6.6),
    ("Ouagadougou", 28.3),
    ("Ouarzazate", 18.9),
    ("Oulu", 2.7),
    ("Palembang", 27.3),
    ("Palermo", 18.5),
    ("Palm Springs", 24.5),
    ("Palmerston North", 13.2),
    ("Panama City", 28.0),
    ("Parakou", 26.8),
    ("Paris", 12.3),
    ("Perth", 18.7),
    ("Petropavlovsk-Kamchatsky", 1.9),
    ("Philadelphia", 13.2),
    ("Phnom Penh", 28.3),
    ("Phoenix", 23.9),
    ("Pittsburgh", 10.8),
    ("Podgorica", 15.3),
    ("Pointe-Noire", 26.1),
    ("Pontianak", 27.7),
    ("Port Moresby", 26.9),
    ("Port Sudan", 28.4),
    ("Port Vila", 24.3),
    ("Port-Gentil", 26.0),
    ("Portland (OR)", 12.4),
    ("Porto", 15.7),
    ("Prague", 8.4),
    ("Praia", 24.4),
    ("Pretoria", 18.2),
    ("Pyongyang", 10.8),
    ("Queenstown", 9.0),
    ("Quito", 15.0),
    ("Rabat", 17.2),
    ("Rangpur", 24.4),
    ("Reykjavik", 4.3),
    ("Riga", 6.2),
    ("Riyadh", 26.0),
    ("Rome", 15.2),
    ("Roseau", 26.2),
    ("Rostov-on-Don", 9.9),
    ("Sacramento", 16.3),
    ("Saint Petersburg", 5.8),
    ("Saint-Pierre", 5.7),
    ("Salt Lake City", 11.6),
    ("San Antonio", 20.8),
    ("San Diego", 17.8),
    ("San Francisco", 14.6),
    ("San Jose", 16.4),
    ("San José", 22.6),
    ("San Juan", 27.2),
    ("San Salvador", 23.1),
    ("Sana'a", 20.0),
    ("Santiago", 14.9),
    ("Santo Domingo", 25.9),
    ("São Paulo", 19.6),
    ("São Tomé", 25.3),
    ("Sapporo", 8.9),
    ("Sarajevo", 10.1),
    ("Saskatoon", 3.3),
    ("Seattle", 11.3),
    ("Seoul", 12.5),
    ("Seville", 19.2),
    ("Shanghai", 16.7),
    ("Singapore", 27.0),
    ("Skopje", 12.4),
    ("Sochi", 14.2),
    ("Sofia", 10.6),
    ("Sokoto", 28.0),
    ("Split", 16.1),
    ("St. John's", 5.0),
    ("St. Louis", 13.9),
    ("Stockholm", 6.6),
    ("Surabaya", 27.1),
    ("Suva", 25.6),
    ("Sydney", 17.7),
    ("Tabora", 23.0),
    ("Tabriz", 12.6),
    ("Taipei", 23.0),
    ("Tallinn", 6.4),
    ("Tamale", 27.9),
    ("Tamanrasset", 21.7),
    ("Tampa", 22.9),
    ("Tashkent", 14.8),
    ("Tbilisi", 12.9),
    ("Tegucigalpa", 21.7),
    ("Tehran", 17.0),
    ("Tel Aviv", 20.0),
    ("Thessaloniki", 16.0),
    ("Thiès", 24.0),
    ("Tijuana", 17.8),
    ("Timbuktu", 28.0),
    ("Tirana", 15.2),
    ("Toamasina", 23.4),
    ("Tokyo", 15.4),
    ("Toliara", 24.1),
    ("Toluca", 12.4),
    ("Toronto", 9.4),
    ("Tripoli", 20.0),
    ("Tromsø", 2.9),
    ("Tucson", 20.9),
    ("Tunis", 18.4),
    ("Ulaanbaatar", -0.4),
    ("Upington", 20.4),
    ("Ürümqi", 7.4),
    ("Vaduz", 10.1),
    ("Valencia", 18.3),
    ("Valletta", 18.8),
    ("Vancouver", 10.4),
    ("Varanasi", 25.3),
    ("Venice", 12.5),
    ("Veracruz", 25.4),
    ("Vienna", 10.4),
    ("Vientiane", 25.9),
    ("Vilnius", 6.0),
    ("Virginia Beach", 15.8),
    ("Vladivostok", 4.9),
    ("Warsaw", 8.5),
    ("Washington, D.C.", 14.6),
    ("Wellington", 12.9),
    ("Whitehorse", -0.1),
    ("Wichita", 13.9),
    ("Willemstad", 28.0),
    ("Winnipeg", 3.0),
    ("Wrocław", 9.6),
    ("Xi'an", 14.1),
    ("Yakutsk", -8.8),
    ("Yangon", 27.5),
    ("Yaoundé", 23.8),
    ("Yellowknife", -4.3),
    ("Yerevan", 12.4),
    ("Yokohama", 15.5),
    ("Zagreb", 10.7),
    ("Zanzibar City", 26.0),
    ("Zürich", 9.3),
];

struct Stats {
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
}

impl Stats {
    fn new() -> Self {
        Self {
            min: f64::MAX,
            max: f64::MIN,
            sum: 0.0,
            count: 0,
        }
    }

    fn update(&mut self, temp: f64) {
        if temp < self.min {
            self.min = temp;
        }
        if temp > self.max {
            self.max = temp;
        }
        self.sum += temp;
        self.count += 1;
    }

    fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }
}

pub fn generate(rows: u64, measurements_path: &str, expected_path: &str) -> Result<()> {
    UI::info(&format!("generating {} rows", rows));

    // create parent directories if needed
    if let Some(parent) = Path::new(measurements_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = Path::new(expected_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(measurements_path)?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);
    let mut rng = rand::thread_rng();

    // track stats per station using BTreeMap for sorted output
    let mut stats: BTreeMap<&str, Stats> = BTreeMap::new();

    let station_count = STATIONS.len();

    for i in 0..rows {
        let (name, mean_temp) = STATIONS[rng.gen_range(0..station_count)];
        let offset: f64 = rng.gen_range(-10.0..10.0);
        let temp = ((mean_temp + offset) * 10.0).round() / 10.0;

        writeln!(writer, "{};{:.1}", name, temp)?;

        stats.entry(name).or_insert_with(Stats::new).update(temp);

        if rows >= 1_000_000 && (i + 1) % 1_000_000 == 0 {
            UI::info(&format!("  {} million rows", (i + 1) / 1_000_000));
        }
    }

    writer.flush()?;

    // write expected output
    let mut expected = String::from("{");
    let mut first = true;
    for (name, s) in &stats {
        if !first {
            expected.push_str(", ");
        }
        first = false;
        expected.push_str(&format!(
            "{}={:.1}/{:.1}/{:.1}",
            name,
            s.min,
            s.mean(),
            s.max
        ));
    }
    expected.push_str("}\n");

    std::fs::write(expected_path, expected)?;

    UI::success(&format!("generated {} rows", rows));
    UI::kv("measurements", measurements_path);
    UI::kv("expected", expected_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_stats_update() {
        let mut s = Stats::new();
        s.update(10.0);
        s.update(20.0);
        s.update(15.0);

        assert_eq!(s.min, 10.0);
        assert_eq!(s.max, 20.0);
        assert_eq!(s.sum, 45.0);
        assert_eq!(s.count, 3);
        assert!((s.mean() - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_generate_creates_files() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let measurements = dir.path().join("measurements.txt");
        let expected = dir.path().join("expected.txt");

        generate(
            100,
            measurements.to_str().unwrap(),
            expected.to_str().unwrap(),
        )
        .expect("generate failed");

        assert!(measurements.exists(), "measurements file should exist");
        assert!(expected.exists(), "expected file should exist");
    }

    #[test]
    fn test_generate_correct_row_count() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let measurements = dir.path().join("measurements.txt");
        let expected = dir.path().join("expected.txt");

        generate(
            50,
            measurements.to_str().unwrap(),
            expected.to_str().unwrap(),
        )
        .expect("generate failed");

        let content = fs::read_to_string(&measurements).expect("failed to read");
        let line_count = content.lines().count();
        assert_eq!(line_count, 50, "should have 50 lines");
    }

    #[test]
    fn test_generate_measurement_format() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let measurements = dir.path().join("measurements.txt");
        let expected = dir.path().join("expected.txt");

        generate(
            10,
            measurements.to_str().unwrap(),
            expected.to_str().unwrap(),
        )
        .expect("generate failed");

        let content = fs::read_to_string(&measurements).expect("failed to read");
        for line in content.lines() {
            assert!(
                line.contains(';'),
                "line should contain semicolon: {}",
                line
            );
            let parts: Vec<&str> = line.split(';').collect();
            assert_eq!(parts.len(), 2, "line should have 2 parts: {}", line);
            // temperature should be parseable
            let temp: f64 = parts[1].parse().expect("temperature should be a number");
            assert!(
                temp > -50.0 && temp < 60.0,
                "temp should be reasonable: {}",
                temp
            );
        }
    }

    #[test]
    fn test_generate_expected_format() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let measurements = dir.path().join("measurements.txt");
        let expected = dir.path().join("expected.txt");

        generate(
            100,
            measurements.to_str().unwrap(),
            expected.to_str().unwrap(),
        )
        .expect("generate failed");

        let content = fs::read_to_string(&expected).expect("failed to read");
        assert!(content.starts_with('{'), "expected should start with {{");
        assert!(content.trim().ends_with('}'), "expected should end with }}");
        // should contain station=min/mean/max format
        assert!(
            content.contains('/'),
            "expected should contain / separators"
        );
    }

    #[test]
    fn test_generate_creates_parent_dirs() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let measurements = dir.path().join("data/nested/measurements.txt");
        let expected = dir.path().join("expected/nested/output.txt");

        generate(
            10,
            measurements.to_str().unwrap(),
            expected.to_str().unwrap(),
        )
        .expect("generate failed");

        assert!(
            measurements.exists(),
            "measurements file should exist in nested dir"
        );
        assert!(
            expected.exists(),
            "expected file should exist in nested dir"
        );
    }

    #[test]
    fn test_stations_not_empty() {
        assert!(!STATIONS.is_empty(), "stations list should not be empty");
        assert!(STATIONS.len() > 100, "should have many stations");
    }
}
