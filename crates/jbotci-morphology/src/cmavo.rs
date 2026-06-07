use std::fmt;

use bityzba::requires;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cmavo {
    A,
    Aha,
    Ahai,
    Ahe,
    Ahi,
    Aho,
    Ahoi,
    Ahu,
    Ahy,
    Ai,
    Aihi,
    Au,
    Auhau,
    Ba,
    Baha,
    Bahau,
    Bahe,
    Bahei,
    Bahi,
    Baho,
    Bahoi,
    Bahu,
    Bai,
    Baihau,
    Bau,
    Be,
    Beha,
    Behau,
    Behe,
    Behei,
    Behi,
    Beho,
    Behu,
    Bei,
    Bi,
    Bihai,
    Bihe,
    Bihi,
    Biho,
    Bihu,
    Bo,
    Bohai,
    Bohei,
    Boi,
    Boihau,
    Bu,
    Buha,
    Buhe,
    Buhei,
    Buhi,
    Buho,
    Buhu,
    Buhuhe,
    By,
    Ca,
    Caha,
    Cahe,
    Cahi,
    Caho,
    Cahu,
    Cai,
    Cau,
    Cauhe,
    Cauhi,
    Ce,
    Ceha,
    Cehai,
    Cehe,
    Cehi,
    Ceho,
    Cehu,
    Cei,
    Ceiha,
    Ceihi,
    Ci,
    Cihe,
    Cihi,
    Ciho,
    Cihu,
    Co,
    Coha,
    Cohaha,
    Cohauha,
    Cohe,
    Cohi,
    Coho,
    Cohoi,
    Cohu,
    Cohuha,
    Coi,
    Cu,
    Cuha,
    Cuhe,
    Cuhei,
    Cuhi,
    Cuho,
    Cuhu,
    Cy,
    Da,
    Daha,
    Dahe,
    Dahei,
    Dahi,
    Daho,
    Dahoi,
    Dahu,
    Dai,
    Daiha,
    Daihe,
    Daihi,
    Daiho,
    Daihu,
    Daihy,
    Dau,
    Dauha,
    Dauhe,
    Dauhi,
    Dauho,
    Dauhu,
    De,
    Deha,
    Dehahu,
    Dehai,
    Dehe,
    Dehei,
    Dehi,
    Dehiha,
    Dehihe,
    Dehihi,
    Dehiho,
    Dehihu,
    Deho,
    Dehoha,
    Dehu,
    Dei,
    Deiha,
    Di,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Diha,
    Dihai,
    Dihe,
    Dihei,
    Dihi,
    Diho,
    Dihu,
    Do,
    Doha,
    Dohai,
    Dohe,
    Dohi,
    Doho,
    Dohu,
    Doi,
    Du,
    Duha,
    Duhe,
    Duhei,
    Duhi,
    Duho,
    Duhoi,
    Duhu,
    Dy,
    E,
    Eha,
    Ehe,
    Ehei,
    Ehi,
    Eho,
    Ehu,
    Ehuhi,
    Ehy,
    Ei,
    Eihai,
    Eihei,
    Fa,
    Faha,
    Fahai,
    Fahe,
    Fahi,
    Faho,
    Fahu,
    Fai,
    Faihu,
    Fau,
    Fauha,
    Fauhe,
    Fauhi,
    Fauho,
    Fauhu,
    Fe,
    Feha,
    Fehaha,
    Fehahe,
    Fehahi,
    Fehaho,
    Fehe,
    Fehi,
    Feho,
    Fehu,
    Fei,
    Feihe,
    Fi,
    Fiha,
    Fihau,
    Fihe,
    Fihi,
    Fiho,
    Fihoi,
    Fihu,
    Fo,
    Foha,
    Fohai,
    Fohe,
    Fohi,
    Foho,
    Fohu,
    Foi,
    Fu,
    Fuha,
    Fuhau,
    Fuhe,
    Fuhi,
    Fuho,
    Fuhu,
    Fy,
    Ga,
    Gaha,
    Gahe,
    Gahei,
    Gahi,
    Gahihi,
    Gaho,
    Gahu,
    Gahuhi,
    Gai,
    Gaiha,
    Gaihe,
    Gaihi,
    Gaiho,
    Gaihu,
    Gau,
    Gauhi,
    Ge,
    Geha,
    Gehai,
    Gehe,
    Gehi,
    Geho,
    Gehu,
    Gehuhi,
    Gei,
    Geiha,
    Gi,
    Giha,
    Gihe,
    Gihi,
    Giho,
    Gihu,
    Go,
    Goha,
    Gohe,
    Gohi,
    Goho,
    Gohoi,
    Gohu,
    Goi,
    Gu,
    Guha,
    Guhe,
    Guhi,
    Guho,
    Guhu,
    Gy,
    I,
    Ia,
    Iahau,
    Iahu,
    Ie,
    Ieho,
    Iha,
    Ihau,
    Ihe,
    Ihei,
    Ihi,
    Ihihi,
    Iho,
    Ihu,
    Ihy,
    Ii,
    Io,
    Iu,
    Iy,
    Ja,
    Jaha,
    Jahai,
    Jahau,
    Jahe,
    Jahei,
    Jahi,
    Jaho,
    Jahohe,
    Jahoho,
    Jahoi,
    Jahui,
    Jai,
    Jau,
    Jauha,
    Jauhe,
    Jauhi,
    Jauho,
    Jauhu,
    Je,
    Jeha,
    Jehau,
    Jehe,
    Jehi,
    Jeho,
    Jehu,
    Jei,
    Jeihe,
    Jeihi,
    Jeiho,
    Ji,
    Jiha,
    Jihai,
    Jihe,
    Jihehe,
    Jihei,
    Jihi,
    Jihiha,
    Jiho,
    Jihohe,
    Jihoho,
    Jihu,
    Jo,
    Joha,
    Johai,
    Johau,
    Johe,
    Johi,
    Johiha,
    Joho,
    Johu,
    Johuhu,
    Joi,
    Joihe,
    Joihi,
    Joiho,
    Joihu,
    Ju,
    Juha,
    Juhe,
    Juhi,
    Juho,
    Juhoi,
    Juhu,
    Jy,
    Ka,
    Kaha,
    Kahai,
    Kahe,
    Kahi,
    Kaho,
    Kahu,
    Kai,
    Kaihai,
    Kaihu,
    Kau,
    Kauha,
    Kauhe,
    Kauhi,
    Kauho,
    Kauhu,
    Ke,
    Keha,
    Kehau,
    Kehe,
    Kehi,
    Kehihai,
    Keho,
    Kehu,
    Kei,
    Ki,
    Kiha,
    Kihaha,
    Kihai,
    Kihe,
    Kiheha,
    Kihi,
    Kihiha,
    Kiho,
    Kihoha,
    Kihohe,
    Kihoi,
    Kihu,
    Kihuha,
    Kihuhe,
    Kihuhi,
    Ko,
    Koha,
    Kohau,
    Kohe,
    Kohi,
    Koho,
    Kohoi,
    Kohu,
    Koi,
    Ku,
    Kuha,
    Kuhau,
    Kuhe,
    Kuhi,
    Kuho,
    Kuhoi,
    Kuhu,
    Ky,
    La,
    Laha,
    Lahai,
    Lahau,
    Lahe,
    Lahei,
    Lahi,
    Laho,
    Lahoho,
    Lahoi,
    Lahu,
    Lai,
    Lau,
    Le,
    Leha,
    Lehai,
    Lehe,
    Lehei,
    Lehi,
    Leho,
    Lehohe,
    Lehu,
    Lei,
    Leihe,
    Leihi,
    Li,
    Liha,
    Lihai,
    Lihau,
    Lihe,
    Lihehe,
    Lihei,
    Lihi,
    Liho,
    Lihoi,
    Lihu,
    Lo,
    Loha,
    Lohai,
    Lohe,
    Lohei,
    Lohi,
    Loho,
    Lohoi,
    Lohu,
    Loi,
    Loihe,
    Loihi,
    Lu,
    Luha,
    Luhe,
    Luhei,
    Luhi,
    Luho,
    Luhu,
    Ly,
    Ma,
    Maha,
    Mahai,
    Mahau,
    Mahe,
    Mahei,
    Mahi,
    Maho,
    Mahoi,
    Mahu,
    Mai,
    Maiho,
    Mau,
    Mauha,
    Mauhe,
    Mauhi,
    Mauho,
    Mauhu,
    Me,
    Meha,
    Mehau,
    Mehe,
    Mehei,
    Mehi,
    Meho,
    Mehohe,
    Mehoi,
    Mehu,
    Mei,
    Mi,
    Miha,
    Mihai,
    Mihau,
    Mihe,
    Mihei,
    Mihi,
    Miho,
    Mihu,
    Mo,
    Moha,
    Mohe,
    Mohi,
    Moho,
    Mohoi,
    Mohu,
    Moi,
    Moiho,
    Moihoi,
    Mu,
    Muha,
    Muhai,
    Muhe,
    Muhei,
    Muhi,
    Muho,
    Muhoi,
    Muhu,
    My,
    Na,
    Naha,
    Nahe,
    Nahei,
    Nahi,
    Naho,
    Nahoi,
    Nahu,
    Nai,
    Nau,
    Nauho,
    Nauhu,
    Ne,
    Neha,
    Nehahi,
    Nehi,
    Neho,
    Nehu,
    Nei,
    Ni,
    Niha,
    Nihe,
    Nihei,
    Nihi,
    Nihihi,
    Niho,
    Nihu,
    No,
    Noha,
    Nohe,
    Nohei,
    Nohi,
    Noho,
    Nohoi,
    Nohu,
    Noi,
    Noiha,
    Noihi,
    Noihoha,
    Nu,
    Nuha,
    Nuhe,
    Nuhi,
    Nuho,
    Nuhu,
    Ny,
    O,
    Oha,
    Ohai,
    Ohe,
    Ohi,
    Oho,
    Ohu,
    Ohy,
    Oi,
    Oiha,
    Oihoi,
    Pa,
    Paha,
    Pahahi,
    Pahe,
    Pahi,
    Paho,
    Pahu,
    Pai,
    Paihe,
    Pau,
    Pe,
    Peha,
    Pehahi,
    Pehe,
    Pehei,
    Pehi,
    Peho,
    Pehu,
    Pei,
    Peihe,
    Pi,
    Piha,
    Pihai,
    Pihe,
    Pihei,
    Pihi,
    Piho,
    Pihu,
    Po,
    Pohai,
    Pohe,
    Pohi,
    Poho,
    Pohoi,
    Pohu,
    Poi,
    Poiha,
    Poihei,
    Poihi,
    Poihoha,
    Pu,
    Puha,
    Puhau,
    Puhe,
    Puhehi,
    Puhi,
    Puhiha,
    Puhihi,
    Puho,
    Puhohi,
    Puhu,
    Py,
    Ra,
    Raha,
    Rahai,
    Rahe,
    Rahi,
    Raho,
    Rahoi,
    Rahu,
    Rai,
    Raihe,
    Rau,
    Rauhi,
    Re,
    Reha,
    Rehe,
    Rehei,
    Rehi,
    Reho,
    Rehu,
    Rei,
    Ri,
    Riha,
    Rihe,
    Rihi,
    Rihiha,
    Rihihe,
    Rihihi,
    Rihiho,
    Rihihu,
    Riho,
    Rihoi,
    Rihu,
    Ro,
    Roha,
    Rohe,
    Rohei,
    Rohi,
    Roho,
    Rohoi,
    Rohu,
    Roi,
    Ru,
    Ruha,
    Ruhe,
    Ruhi,
    Ruho,
    Ruhu,
    Ry,
    Sa,
    Saha,
    Sahai,
    Sahe,
    Sahei,
    Sahi,
    Sahiha,
    Saho,
    Sahu,
    Sai,
    Saihe,
    Saihi,
    Sau,
    Sauha,
    Se,
    Seha,
    Sehe,
    Sehi,
    Seho,
    Sehu,
    Sei,
    Seiha,
    Seihe,
    Seihi,
    Si,
    Siha,
    Sihau,
    Sihe,
    Sihi,
    Siho,
    Sihu,
    So,
    Soha,
    Sohahu,
    Sohai,
    Sohe,
    Sohei,
    Sohi,
    Soho,
    Sohoi,
    Sohu,
    Soi,
    Soiha,
    Soihe,
    Su,
    Suha,
    Suhai,
    Suhe,
    Suhei,
    Suhi,
    Suho,
    Suhoi,
    Suhu,
    Sy,
    Ta,
    Taha,
    Tahai,
    Tahe,
    Tahi,
    Tahiha,
    Tahihe,
    Tahihi,
    Tahiho,
    Tahihu,
    Taho,
    Tahu,
    Tahuhi,
    Tai,
    Tau,
    Te,
    Teha,
    Tehai,
    Tehe,
    Teho,
    Tehoi,
    Tehu,
    Tei,
    Ti,
    Tiha,
    Tihau,
    Tihe,
    Tihi,
    Tihiha,
    Tiho,
    Tihu,
    Tihuha,
    Tihuhi,
    Tihuhu,
    To,
    Toha,
    Tohai,
    Tohe,
    Tohi,
    Toho,
    Tohohe,
    Tohu,
    Toi,
    Tu,
    Tuha,
    Tuhai,
    Tuhau,
    Tuhe,
    Tuhi,
    Tuhiha,
    Tuhihe,
    Tuhihi,
    Tuhiho,
    Tuhihu,
    Tuho,
    Tuhu,
    Ty,
    U,
    Ua,
    Ue,
    Uehi,
    Uha,
    Uhe,
    Uhi,
    Uho,
    Uhohe,
    Uhohi,
    Uhoho,
    Uhohu,
    Uhoi,
    Uhu,
    Uhy,
    Ui,
    Uihai,
    Uo,
    Uu,
    Uy,
    Va,
    Vaha,
    Vahe,
    Vahei,
    Vahi,
    Vaho,
    Vahohi,
    Vahu,
    Vai,
    Vaihe,
    Vau,
    Ve,
    Veha,
    Vehe,
    Vehi,
    Veho,
    Vehu,
    Vei,
    Vi,
    Viha,
    Vihe,
    Vihi,
    Viho,
    Vihu,
    Vo,
    Voha,
    Vohai,
    Vohe,
    Vohi,
    Voho,
    Vohu,
    Voi,
    Voihe,
    Voihi,
    Vu,
    Vuha,
    Vuhe,
    Vuhi,
    Vuho,
    Vuhu,
    Vy,
    Xa,
    Xaho,
    Xai,
    Xaihe,
    Xauha,
    Xauhe,
    Xauhi,
    Xauho,
    Xauhu,
    Xe,
    Xehau,
    Xehe,
    Xehei,
    Xehiha,
    Xehihe,
    Xehihi,
    Xehiho,
    Xehihu,
    Xehu,
    Xeihe,
    Xi,
    Xihe,
    Xihi,
    Xo,
    Xohai,
    Xohe,
    Xohi,
    Xoho,
    Xohu,
    Xoi,
    Xoihi,
    Xu,
    Xuhai,
    Xuhau,
    Xuhei,
    Xuhu,
    Xy,
    Y,
    Yhy,
    Za,
    Zaha,
    Zahai,
    Zahe,
    Zahei,
    Zahi,
    Zaho,
    Zahoha,
    Zahu,
    Zai,
    Zau,
    Zauha,
    Zauhe,
    Zauhi,
    Zauho,
    Zauhu,
    Ze,
    Zeha,
    Zehe,
    Zehi,
    Zeho,
    Zehoi,
    Zehu,
    Zei,
    Zi,
    Zihe,
    Ziho,
    Zo,
    Zoha,
    Zohau,
    Zohe,
    Zohei,
    Zohi,
    Zoho,
    Zohoi,
    Zohu,
    Zoi,
    Zu,
    Zuha,
    Zuhai,
    Zuhau,
    Zuhe,
    Zuhi,
    Zuho,
    Zuhu,
    Zy,
}

impl Cmavo {
    #[requires(true)]
    #[bityzba::ensures(ret.is_some() -> !text.is_empty())]
    pub fn from_text(text: &str) -> Option<Self> {
        Some(match crate::canonicalize_text(text).as_str() {
            "0" => Self::Digit0,
            "1" => Self::Digit1,
            "2" => Self::Digit2,
            "3" => Self::Digit3,
            "4" => Self::Digit4,
            "5" => Self::Digit5,
            "6" => Self::Digit6,
            "7" => Self::Digit7,
            "8" => Self::Digit8,
            "9" => Self::Digit9,
            "a" => Self::A,
            "a'a" => Self::Aha,
            "a'ai" => Self::Ahai,
            "a'e" => Self::Ahe,
            "a'i" => Self::Ahi,
            "a'o" => Self::Aho,
            "a'oi" => Self::Ahoi,
            "a'u" => Self::Ahu,
            "a'y" => Self::Ahy,
            "ai" => Self::Ai,
            "ai'i" => Self::Aihi,
            "au" => Self::Au,
            "au'au" => Self::Auhau,
            "ba" => Self::Ba,
            "ba'a" => Self::Baha,
            "ba'au" => Self::Bahau,
            "ba'e" => Self::Bahe,
            "ba'ei" => Self::Bahei,
            "ba'i" => Self::Bahi,
            "ba'o" => Self::Baho,
            "ba'oi" => Self::Bahoi,
            "ba'u" => Self::Bahu,
            "bai" => Self::Bai,
            "bai'au" => Self::Baihau,
            "bau" => Self::Bau,
            "be" => Self::Be,
            "be'a" => Self::Beha,
            "be'au" => Self::Behau,
            "be'e" => Self::Behe,
            "be'ei" => Self::Behei,
            "be'i" => Self::Behi,
            "be'o" => Self::Beho,
            "be'u" => Self::Behu,
            "bei" => Self::Bei,
            "bi" => Self::Bi,
            "bi'ai" => Self::Bihai,
            "bi'e" => Self::Bihe,
            "bi'i" => Self::Bihi,
            "bi'o" => Self::Biho,
            "bi'u" => Self::Bihu,
            "bo" => Self::Bo,
            "bo'ai" => Self::Bohai,
            "bo'ei" => Self::Bohei,
            "boi" => Self::Boi,
            "boi'au" => Self::Boihau,
            "bu" => Self::Bu,
            "bu'a" => Self::Buha,
            "bu'e" => Self::Buhe,
            "bu'ei" => Self::Buhei,
            "bu'i" => Self::Buhi,
            "bu'o" => Self::Buho,
            "bu'u" => Self::Buhu,
            "bu'u'e" => Self::Buhuhe,
            "by" => Self::By,
            "ca" => Self::Ca,
            "ca'a" => Self::Caha,
            "ca'e" => Self::Cahe,
            "ca'i" => Self::Cahi,
            "ca'o" => Self::Caho,
            "ca'u" => Self::Cahu,
            "cai" => Self::Cai,
            "cau" => Self::Cau,
            "cau'e" => Self::Cauhe,
            "cau'i" => Self::Cauhi,
            "ce" => Self::Ce,
            "ce'a" => Self::Ceha,
            "ce'ai" => Self::Cehai,
            "ce'e" => Self::Cehe,
            "ce'i" => Self::Cehi,
            "ce'o" => Self::Ceho,
            "ce'u" => Self::Cehu,
            "cei" => Self::Cei,
            "cei'a" => Self::Ceiha,
            "cei'i" => Self::Ceihi,
            "ci" => Self::Ci,
            "ci'e" => Self::Cihe,
            "ci'i" => Self::Cihi,
            "ci'o" => Self::Ciho,
            "ci'u" => Self::Cihu,
            "co" => Self::Co,
            "co'a" => Self::Coha,
            "co'a'a" => Self::Cohaha,
            "co'au'a" => Self::Cohauha,
            "co'e" => Self::Cohe,
            "co'i" => Self::Cohi,
            "co'o" => Self::Coho,
            "co'oi" => Self::Cohoi,
            "co'u" => Self::Cohu,
            "co'u'a" => Self::Cohuha,
            "coi" => Self::Coi,
            "cu" => Self::Cu,
            "cu'a" => Self::Cuha,
            "cu'e" => Self::Cuhe,
            "cu'ei" => Self::Cuhei,
            "cu'i" => Self::Cuhi,
            "cu'o" => Self::Cuho,
            "cu'u" => Self::Cuhu,
            "cy" => Self::Cy,
            "da" => Self::Da,
            "da'a" => Self::Daha,
            "da'e" => Self::Dahe,
            "da'ei" => Self::Dahei,
            "da'i" => Self::Dahi,
            "da'o" => Self::Daho,
            "da'oi" => Self::Dahoi,
            "da'u" => Self::Dahu,
            "dai" => Self::Dai,
            "dai'a" => Self::Daiha,
            "dai'e" => Self::Daihe,
            "dai'i" => Self::Daihi,
            "dai'o" => Self::Daiho,
            "dai'u" => Self::Daihu,
            "dai'y" => Self::Daihy,
            "dau" => Self::Dau,
            "dau'a" => Self::Dauha,
            "dau'e" => Self::Dauhe,
            "dau'i" => Self::Dauhi,
            "dau'o" => Self::Dauho,
            "dau'u" => Self::Dauhu,
            "de" => Self::De,
            "de'a" => Self::Deha,
            "de'a'u" => Self::Dehahu,
            "de'ai" => Self::Dehai,
            "de'e" => Self::Dehe,
            "de'ei" => Self::Dehei,
            "de'i" => Self::Dehi,
            "de'i'a" => Self::Dehiha,
            "de'i'e" => Self::Dehihe,
            "de'i'i" => Self::Dehihi,
            "de'i'o" => Self::Dehiho,
            "de'i'u" => Self::Dehihu,
            "de'o" => Self::Deho,
            "de'o'a" => Self::Dehoha,
            "de'u" => Self::Dehu,
            "dei" => Self::Dei,
            "dei'a" => Self::Deiha,
            "di" => Self::Di,
            "di'a" => Self::Diha,
            "di'ai" => Self::Dihai,
            "di'e" => Self::Dihe,
            "di'ei" => Self::Dihei,
            "di'i" => Self::Dihi,
            "di'o" => Self::Diho,
            "di'u" => Self::Dihu,
            "do" => Self::Do,
            "do'a" => Self::Doha,
            "do'ai" => Self::Dohai,
            "do'e" => Self::Dohe,
            "do'i" => Self::Dohi,
            "do'o" => Self::Doho,
            "do'u" => Self::Dohu,
            "doi" => Self::Doi,
            "du" => Self::Du,
            "du'a" => Self::Duha,
            "du'e" => Self::Duhe,
            "du'ei" => Self::Duhei,
            "du'i" => Self::Duhi,
            "du'o" => Self::Duho,
            "du'oi" => Self::Duhoi,
            "du'u" => Self::Duhu,
            "dy" => Self::Dy,
            "e" => Self::E,
            "e'a" => Self::Eha,
            "e'e" => Self::Ehe,
            "e'ei" => Self::Ehei,
            "e'i" => Self::Ehi,
            "e'o" => Self::Eho,
            "e'u" => Self::Ehu,
            "e'u'i" => Self::Ehuhi,
            "e'y" => Self::Ehy,
            "ei" => Self::Ei,
            "ei'ai" => Self::Eihai,
            "ei'ei" => Self::Eihei,
            "fa" => Self::Fa,
            "fa'a" => Self::Faha,
            "fa'ai" => Self::Fahai,
            "fa'e" => Self::Fahe,
            "fa'i" => Self::Fahi,
            "fa'o" => Self::Faho,
            "fa'u" => Self::Fahu,
            "fai" => Self::Fai,
            "fai'u" => Self::Faihu,
            "fau" => Self::Fau,
            "fau'a" => Self::Fauha,
            "fau'e" => Self::Fauhe,
            "fau'i" => Self::Fauhi,
            "fau'o" => Self::Fauho,
            "fau'u" => Self::Fauhu,
            "fe" => Self::Fe,
            "fe'a" => Self::Feha,
            "fe'a'a" => Self::Fehaha,
            "fe'a'e" => Self::Fehahe,
            "fe'a'i" => Self::Fehahi,
            "fe'a'o" => Self::Fehaho,
            "fe'e" => Self::Fehe,
            "fe'i" => Self::Fehi,
            "fe'o" => Self::Feho,
            "fe'u" => Self::Fehu,
            "fei" => Self::Fei,
            "fei'e" => Self::Feihe,
            "fi" => Self::Fi,
            "fi'a" => Self::Fiha,
            "fi'au" => Self::Fihau,
            "fi'e" => Self::Fihe,
            "fi'i" => Self::Fihi,
            "fi'o" => Self::Fiho,
            "fi'oi" => Self::Fihoi,
            "fi'u" => Self::Fihu,
            "fo" => Self::Fo,
            "fo'a" => Self::Foha,
            "fo'ai" => Self::Fohai,
            "fo'e" => Self::Fohe,
            "fo'i" => Self::Fohi,
            "fo'o" => Self::Foho,
            "fo'u" => Self::Fohu,
            "foi" => Self::Foi,
            "fu" => Self::Fu,
            "fu'a" => Self::Fuha,
            "fu'au" => Self::Fuhau,
            "fu'e" => Self::Fuhe,
            "fu'i" => Self::Fuhi,
            "fu'o" => Self::Fuho,
            "fu'u" => Self::Fuhu,
            "fy" => Self::Fy,
            "ga" => Self::Ga,
            "ga'a" => Self::Gaha,
            "ga'e" => Self::Gahe,
            "ga'ei" => Self::Gahei,
            "ga'i" => Self::Gahi,
            "ga'i'i" => Self::Gahihi,
            "ga'o" => Self::Gaho,
            "ga'u" => Self::Gahu,
            "ga'u'i" => Self::Gahuhi,
            "gai" => Self::Gai,
            "gai'a" => Self::Gaiha,
            "gai'e" => Self::Gaihe,
            "gai'i" => Self::Gaihi,
            "gai'o" => Self::Gaiho,
            "gai'u" => Self::Gaihu,
            "gau" => Self::Gau,
            "gau'i" => Self::Gauhi,
            "ge" => Self::Ge,
            "ge'a" => Self::Geha,
            "ge'ai" => Self::Gehai,
            "ge'e" => Self::Gehe,
            "ge'i" => Self::Gehi,
            "ge'o" => Self::Geho,
            "ge'u" => Self::Gehu,
            "ge'u'i" => Self::Gehuhi,
            "gei" => Self::Gei,
            "gei'a" => Self::Geiha,
            "gi" => Self::Gi,
            "gi'a" => Self::Giha,
            "gi'e" => Self::Gihe,
            "gi'i" => Self::Gihi,
            "gi'o" => Self::Giho,
            "gi'u" => Self::Gihu,
            "go" => Self::Go,
            "go'a" => Self::Goha,
            "go'e" => Self::Gohe,
            "go'i" => Self::Gohi,
            "go'o" => Self::Goho,
            "go'oi" => Self::Gohoi,
            "go'u" => Self::Gohu,
            "goi" => Self::Goi,
            "gu" => Self::Gu,
            "gu'a" => Self::Guha,
            "gu'e" => Self::Guhe,
            "gu'i" => Self::Guhi,
            "gu'o" => Self::Guho,
            "gu'u" => Self::Guhu,
            "gy" => Self::Gy,
            "i" => Self::I,
            "i'a" => Self::Iha,
            "i'au" => Self::Ihau,
            "i'e" => Self::Ihe,
            "i'ei" => Self::Ihei,
            "i'i" => Self::Ihi,
            "i'i'i" => Self::Ihihi,
            "i'o" => Self::Iho,
            "i'u" => Self::Ihu,
            "i'y" => Self::Ihy,
            "ia" => Self::Ia,
            "ia'au" => Self::Iahau,
            "ia'u" => Self::Iahu,
            "ie" => Self::Ie,
            "ie'o" => Self::Ieho,
            "ii" => Self::Ii,
            "io" => Self::Io,
            "iu" => Self::Iu,
            "iy" => Self::Iy,
            "ja" => Self::Ja,
            "ja'a" => Self::Jaha,
            "ja'ai" => Self::Jahai,
            "ja'au" => Self::Jahau,
            "ja'e" => Self::Jahe,
            "ja'ei" => Self::Jahei,
            "ja'i" => Self::Jahi,
            "ja'o" => Self::Jaho,
            "ja'o'e" => Self::Jahohe,
            "ja'o'o" => Self::Jahoho,
            "ja'oi" => Self::Jahoi,
            "ja'ui" => Self::Jahui,
            "jai" => Self::Jai,
            "jau" => Self::Jau,
            "jau'a" => Self::Jauha,
            "jau'e" => Self::Jauhe,
            "jau'i" => Self::Jauhi,
            "jau'o" => Self::Jauho,
            "jau'u" => Self::Jauhu,
            "je" => Self::Je,
            "je'a" => Self::Jeha,
            "je'au" => Self::Jehau,
            "je'e" => Self::Jehe,
            "je'i" => Self::Jehi,
            "je'o" => Self::Jeho,
            "je'u" => Self::Jehu,
            "jei" => Self::Jei,
            "jei'e" => Self::Jeihe,
            "jei'i" => Self::Jeihi,
            "jei'o" => Self::Jeiho,
            "ji" => Self::Ji,
            "ji'a" => Self::Jiha,
            "ji'ai" => Self::Jihai,
            "ji'e" => Self::Jihe,
            "ji'e'e" => Self::Jihehe,
            "ji'ei" => Self::Jihei,
            "ji'i" => Self::Jihi,
            "ji'i'a" => Self::Jihiha,
            "ji'o" => Self::Jiho,
            "ji'o'e" => Self::Jihohe,
            "ji'o'o" => Self::Jihoho,
            "ji'u" => Self::Jihu,
            "jo" => Self::Jo,
            "jo'a" => Self::Joha,
            "jo'ai" => Self::Johai,
            "jo'au" => Self::Johau,
            "jo'e" => Self::Johe,
            "jo'i" => Self::Johi,
            "jo'i'a" => Self::Johiha,
            "jo'o" => Self::Joho,
            "jo'u" => Self::Johu,
            "jo'u'u" => Self::Johuhu,
            "joi" => Self::Joi,
            "joi'e" => Self::Joihe,
            "joi'i" => Self::Joihi,
            "joi'o" => Self::Joiho,
            "joi'u" => Self::Joihu,
            "ju" => Self::Ju,
            "ju'a" => Self::Juha,
            "ju'e" => Self::Juhe,
            "ju'i" => Self::Juhi,
            "ju'o" => Self::Juho,
            "ju'oi" => Self::Juhoi,
            "ju'u" => Self::Juhu,
            "jy" => Self::Jy,
            "ka" => Self::Ka,
            "ka'a" => Self::Kaha,
            "ka'ai" => Self::Kahai,
            "ka'e" => Self::Kahe,
            "ka'i" => Self::Kahi,
            "ka'o" => Self::Kaho,
            "ka'u" => Self::Kahu,
            "kai" => Self::Kai,
            "kai'ai" => Self::Kaihai,
            "kai'u" => Self::Kaihu,
            "kau" => Self::Kau,
            "kau'a" => Self::Kauha,
            "kau'e" => Self::Kauhe,
            "kau'i" => Self::Kauhi,
            "kau'o" => Self::Kauho,
            "kau'u" => Self::Kauhu,
            "ke" => Self::Ke,
            "ke'a" => Self::Keha,
            "ke'au" => Self::Kehau,
            "ke'e" => Self::Kehe,
            "ke'i" => Self::Kehi,
            "ke'i'ai" => Self::Kehihai,
            "ke'o" => Self::Keho,
            "ke'u" => Self::Kehu,
            "kei" => Self::Kei,
            "ki" => Self::Ki,
            "ki'a" => Self::Kiha,
            "ki'a'a" => Self::Kihaha,
            "ki'ai" => Self::Kihai,
            "ki'e" => Self::Kihe,
            "ki'e'a" => Self::Kiheha,
            "ki'i" => Self::Kihi,
            "ki'i'a" => Self::Kihiha,
            "ki'o" => Self::Kiho,
            "ki'o'a" => Self::Kihoha,
            "ki'o'e" => Self::Kihohe,
            "ki'oi" => Self::Kihoi,
            "ki'u" => Self::Kihu,
            "ki'u'a" => Self::Kihuha,
            "ki'u'e" => Self::Kihuhe,
            "ki'u'i" => Self::Kihuhi,
            "ko" => Self::Ko,
            "ko'a" => Self::Koha,
            "ko'au" => Self::Kohau,
            "ko'e" => Self::Kohe,
            "ko'i" => Self::Kohi,
            "ko'o" => Self::Koho,
            "ko'oi" => Self::Kohoi,
            "ko'u" => Self::Kohu,
            "koi" => Self::Koi,
            "ku" => Self::Ku,
            "ku'a" => Self::Kuha,
            "ku'au" => Self::Kuhau,
            "ku'e" => Self::Kuhe,
            "ku'i" => Self::Kuhi,
            "ku'o" => Self::Kuho,
            "ku'oi" => Self::Kuhoi,
            "ku'u" => Self::Kuhu,
            "ky" => Self::Ky,
            "la" => Self::La,
            "la'a" => Self::Laha,
            "la'ai" => Self::Lahai,
            "la'au" => Self::Lahau,
            "la'e" => Self::Lahe,
            "la'ei" => Self::Lahei,
            "la'i" => Self::Lahi,
            "la'o" => Self::Laho,
            "la'o'o" => Self::Lahoho,
            "la'oi" => Self::Lahoi,
            "la'u" => Self::Lahu,
            "lai" => Self::Lai,
            "lau" => Self::Lau,
            "le" => Self::Le,
            "le'a" => Self::Leha,
            "le'ai" => Self::Lehai,
            "le'e" => Self::Lehe,
            "le'ei" => Self::Lehei,
            "le'i" => Self::Lehi,
            "le'o" => Self::Leho,
            "le'o'e" => Self::Lehohe,
            "le'u" => Self::Lehu,
            "lei" => Self::Lei,
            "lei'e" => Self::Leihe,
            "lei'i" => Self::Leihi,
            "li" => Self::Li,
            "li'a" => Self::Liha,
            "li'ai" => Self::Lihai,
            "li'au" => Self::Lihau,
            "li'e" => Self::Lihe,
            "li'e'e" => Self::Lihehe,
            "li'ei" => Self::Lihei,
            "li'i" => Self::Lihi,
            "li'o" => Self::Liho,
            "li'oi" => Self::Lihoi,
            "li'u" => Self::Lihu,
            "lo" => Self::Lo,
            "lo'a" => Self::Loha,
            "lo'ai" => Self::Lohai,
            "lo'e" => Self::Lohe,
            "lo'ei" => Self::Lohei,
            "lo'i" => Self::Lohi,
            "lo'o" => Self::Loho,
            "lo'oi" => Self::Lohoi,
            "lo'u" => Self::Lohu,
            "loi" => Self::Loi,
            "loi'e" => Self::Loihe,
            "loi'i" => Self::Loihi,
            "lu" => Self::Lu,
            "lu'a" => Self::Luha,
            "lu'e" => Self::Luhe,
            "lu'ei" => Self::Luhei,
            "lu'i" => Self::Luhi,
            "lu'o" => Self::Luho,
            "lu'u" => Self::Luhu,
            "ly" => Self::Ly,
            "ma" => Self::Ma,
            "ma'a" => Self::Maha,
            "ma'ai" => Self::Mahai,
            "ma'au" => Self::Mahau,
            "ma'e" => Self::Mahe,
            "ma'ei" => Self::Mahei,
            "ma'i" => Self::Mahi,
            "ma'o" => Self::Maho,
            "ma'oi" => Self::Mahoi,
            "ma'u" => Self::Mahu,
            "mai" => Self::Mai,
            "mai'o" => Self::Maiho,
            "mau" => Self::Mau,
            "mau'a" => Self::Mauha,
            "mau'e" => Self::Mauhe,
            "mau'i" => Self::Mauhi,
            "mau'o" => Self::Mauho,
            "mau'u" => Self::Mauhu,
            "me" => Self::Me,
            "me'a" => Self::Meha,
            "me'au" => Self::Mehau,
            "me'e" => Self::Mehe,
            "me'ei" => Self::Mehei,
            "me'i" => Self::Mehi,
            "me'o" => Self::Meho,
            "me'o'e" => Self::Mehohe,
            "me'oi" => Self::Mehoi,
            "me'u" => Self::Mehu,
            "mei" => Self::Mei,
            "mi" => Self::Mi,
            "mi'a" => Self::Miha,
            "mi'ai" => Self::Mihai,
            "mi'au" => Self::Mihau,
            "mi'e" => Self::Mihe,
            "mi'ei" => Self::Mihei,
            "mi'i" => Self::Mihi,
            "mi'o" => Self::Miho,
            "mi'u" => Self::Mihu,
            "mo" => Self::Mo,
            "mo'a" => Self::Moha,
            "mo'e" => Self::Mohe,
            "mo'i" => Self::Mohi,
            "mo'o" => Self::Moho,
            "mo'oi" => Self::Mohoi,
            "mo'u" => Self::Mohu,
            "moi" => Self::Moi,
            "moi'o" => Self::Moiho,
            "moi'oi" => Self::Moihoi,
            "mu" => Self::Mu,
            "mu'a" => Self::Muha,
            "mu'ai" => Self::Muhai,
            "mu'e" => Self::Muhe,
            "mu'ei" => Self::Muhei,
            "mu'i" => Self::Muhi,
            "mu'o" => Self::Muho,
            "mu'oi" => Self::Muhoi,
            "mu'u" => Self::Muhu,
            "my" => Self::My,
            "na" => Self::Na,
            "na'a" => Self::Naha,
            "na'e" => Self::Nahe,
            "na'ei" => Self::Nahei,
            "na'i" => Self::Nahi,
            "na'o" => Self::Naho,
            "na'oi" => Self::Nahoi,
            "na'u" => Self::Nahu,
            "nai" => Self::Nai,
            "nau" => Self::Nau,
            "nau'o" => Self::Nauho,
            "nau'u" => Self::Nauhu,
            "ne" => Self::Ne,
            "ne'a" => Self::Neha,
            "ne'a'i" => Self::Nehahi,
            "ne'i" => Self::Nehi,
            "ne'o" => Self::Neho,
            "ne'u" => Self::Nehu,
            "nei" => Self::Nei,
            "ni" => Self::Ni,
            "ni'a" => Self::Niha,
            "ni'e" => Self::Nihe,
            "ni'ei" => Self::Nihei,
            "ni'i" => Self::Nihi,
            "ni'i'i" => Self::Nihihi,
            "ni'o" => Self::Niho,
            "ni'u" => Self::Nihu,
            "no" => Self::No,
            "no'a" => Self::Noha,
            "no'e" => Self::Nohe,
            "no'ei" => Self::Nohei,
            "no'i" => Self::Nohi,
            "no'o" => Self::Noho,
            "no'oi" => Self::Nohoi,
            "no'u" => Self::Nohu,
            "noi" => Self::Noi,
            "noi'a" => Self::Noiha,
            "noi'i" => Self::Noihi,
            "noi'o'a" => Self::Noihoha,
            "nu" => Self::Nu,
            "nu'a" => Self::Nuha,
            "nu'e" => Self::Nuhe,
            "nu'i" => Self::Nuhi,
            "nu'o" => Self::Nuho,
            "nu'u" => Self::Nuhu,
            "ny" => Self::Ny,
            "o" => Self::O,
            "o'a" => Self::Oha,
            "o'ai" => Self::Ohai,
            "o'e" => Self::Ohe,
            "o'i" => Self::Ohi,
            "o'o" => Self::Oho,
            "o'u" => Self::Ohu,
            "o'y" => Self::Ohy,
            "oi" => Self::Oi,
            "oi'a" => Self::Oiha,
            "oi'oi" => Self::Oihoi,
            "pa" => Self::Pa,
            "pa'a" => Self::Paha,
            "pa'a'i" => Self::Pahahi,
            "pa'e" => Self::Pahe,
            "pa'i" => Self::Pahi,
            "pa'o" => Self::Paho,
            "pa'u" => Self::Pahu,
            "pai" => Self::Pai,
            "pai'e" => Self::Paihe,
            "pau" => Self::Pau,
            "pe" => Self::Pe,
            "pe'a" => Self::Peha,
            "pe'a'i" => Self::Pehahi,
            "pe'e" => Self::Pehe,
            "pe'ei" => Self::Pehei,
            "pe'i" => Self::Pehi,
            "pe'o" => Self::Peho,
            "pe'u" => Self::Pehu,
            "pei" => Self::Pei,
            "pei'e" => Self::Peihe,
            "pi" => Self::Pi,
            "pi'a" => Self::Piha,
            "pi'ai" => Self::Pihai,
            "pi'e" => Self::Pihe,
            "pi'ei" => Self::Pihei,
            "pi'i" => Self::Pihi,
            "pi'o" => Self::Piho,
            "pi'u" => Self::Pihu,
            "po" => Self::Po,
            "po'ai" => Self::Pohai,
            "po'e" => Self::Pohe,
            "po'i" => Self::Pohi,
            "po'o" => Self::Poho,
            "po'oi" => Self::Pohoi,
            "po'u" => Self::Pohu,
            "poi" => Self::Poi,
            "poi'a" => Self::Poiha,
            "poi'ei" => Self::Poihei,
            "poi'i" => Self::Poihi,
            "poi'o'a" => Self::Poihoha,
            "pu" => Self::Pu,
            "pu'a" => Self::Puha,
            "pu'au" => Self::Puhau,
            "pu'e" => Self::Puhe,
            "pu'e'i" => Self::Puhehi,
            "pu'i" => Self::Puhi,
            "pu'i'a" => Self::Puhiha,
            "pu'i'i" => Self::Puhihi,
            "pu'o" => Self::Puho,
            "pu'o'i" => Self::Puhohi,
            "pu'u" => Self::Puhu,
            "py" => Self::Py,
            "ra" => Self::Ra,
            "ra'a" => Self::Raha,
            "ra'ai" => Self::Rahai,
            "ra'e" => Self::Rahe,
            "ra'i" => Self::Rahi,
            "ra'o" => Self::Raho,
            "ra'oi" => Self::Rahoi,
            "ra'u" => Self::Rahu,
            "rai" => Self::Rai,
            "rai'e" => Self::Raihe,
            "rau" => Self::Rau,
            "rau'i" => Self::Rauhi,
            "re" => Self::Re,
            "re'a" => Self::Reha,
            "re'e" => Self::Rehe,
            "re'ei" => Self::Rehei,
            "re'i" => Self::Rehi,
            "re'o" => Self::Reho,
            "re'u" => Self::Rehu,
            "rei" => Self::Rei,
            "ri" => Self::Ri,
            "ri'a" => Self::Riha,
            "ri'e" => Self::Rihe,
            "ri'i" => Self::Rihi,
            "ri'i'a" => Self::Rihiha,
            "ri'i'e" => Self::Rihihe,
            "ri'i'i" => Self::Rihihi,
            "ri'i'o" => Self::Rihiho,
            "ri'i'u" => Self::Rihihu,
            "ri'o" => Self::Riho,
            "ri'oi" => Self::Rihoi,
            "ri'u" => Self::Rihu,
            "ro" => Self::Ro,
            "ro'a" => Self::Roha,
            "ro'e" => Self::Rohe,
            "ro'ei" => Self::Rohei,
            "ro'i" => Self::Rohi,
            "ro'o" => Self::Roho,
            "ro'oi" => Self::Rohoi,
            "ro'u" => Self::Rohu,
            "roi" => Self::Roi,
            "ru" => Self::Ru,
            "ru'a" => Self::Ruha,
            "ru'e" => Self::Ruhe,
            "ru'i" => Self::Ruhi,
            "ru'o" => Self::Ruho,
            "ru'u" => Self::Ruhu,
            "ry" => Self::Ry,
            "sa" => Self::Sa,
            "sa'a" => Self::Saha,
            "sa'ai" => Self::Sahai,
            "sa'e" => Self::Sahe,
            "sa'ei" => Self::Sahei,
            "sa'i" => Self::Sahi,
            "sa'i'a" => Self::Sahiha,
            "sa'o" => Self::Saho,
            "sa'u" => Self::Sahu,
            "sai" => Self::Sai,
            "sai'e" => Self::Saihe,
            "sai'i" => Self::Saihi,
            "sau" => Self::Sau,
            "sau'a" => Self::Sauha,
            "se" => Self::Se,
            "se'a" => Self::Seha,
            "se'e" => Self::Sehe,
            "se'i" => Self::Sehi,
            "se'o" => Self::Seho,
            "se'u" => Self::Sehu,
            "sei" => Self::Sei,
            "sei'a" => Self::Seiha,
            "sei'e" => Self::Seihe,
            "sei'i" => Self::Seihi,
            "si" => Self::Si,
            "si'a" => Self::Siha,
            "si'au" => Self::Sihau,
            "si'e" => Self::Sihe,
            "si'i" => Self::Sihi,
            "si'o" => Self::Siho,
            "si'u" => Self::Sihu,
            "so" => Self::So,
            "so'a" => Self::Soha,
            "so'a'u" => Self::Sohahu,
            "so'ai" => Self::Sohai,
            "so'e" => Self::Sohe,
            "so'ei" => Self::Sohei,
            "so'i" => Self::Sohi,
            "so'o" => Self::Soho,
            "so'oi" => Self::Sohoi,
            "so'u" => Self::Sohu,
            "soi" => Self::Soi,
            "soi'a" => Self::Soiha,
            "soi'e" => Self::Soihe,
            "su" => Self::Su,
            "su'a" => Self::Suha,
            "su'ai" => Self::Suhai,
            "su'e" => Self::Suhe,
            "su'ei" => Self::Suhei,
            "su'i" => Self::Suhi,
            "su'o" => Self::Suho,
            "su'oi" => Self::Suhoi,
            "su'u" => Self::Suhu,
            "sy" => Self::Sy,
            "ta" => Self::Ta,
            "ta'a" => Self::Taha,
            "ta'ai" => Self::Tahai,
            "ta'e" => Self::Tahe,
            "ta'i" => Self::Tahi,
            "ta'i'a" => Self::Tahiha,
            "ta'i'e" => Self::Tahihe,
            "ta'i'i" => Self::Tahihi,
            "ta'i'o" => Self::Tahiho,
            "ta'i'u" => Self::Tahihu,
            "ta'o" => Self::Taho,
            "ta'u" => Self::Tahu,
            "ta'u'i" => Self::Tahuhi,
            "tai" => Self::Tai,
            "tau" => Self::Tau,
            "te" => Self::Te,
            "te'a" => Self::Teha,
            "te'ai" => Self::Tehai,
            "te'e" => Self::Tehe,
            "te'o" => Self::Teho,
            "te'oi" => Self::Tehoi,
            "te'u" => Self::Tehu,
            "tei" => Self::Tei,
            "ti" => Self::Ti,
            "ti'a" => Self::Tiha,
            "ti'au" => Self::Tihau,
            "ti'e" => Self::Tihe,
            "ti'i" => Self::Tihi,
            "ti'i'a" => Self::Tihiha,
            "ti'o" => Self::Tiho,
            "ti'u" => Self::Tihu,
            "ti'u'a" => Self::Tihuha,
            "ti'u'i" => Self::Tihuhi,
            "ti'u'u" => Self::Tihuhu,
            "to" => Self::To,
            "to'a" => Self::Toha,
            "to'ai" => Self::Tohai,
            "to'e" => Self::Tohe,
            "to'i" => Self::Tohi,
            "to'o" => Self::Toho,
            "to'o'e" => Self::Tohohe,
            "to'u" => Self::Tohu,
            "toi" => Self::Toi,
            "tu" => Self::Tu,
            "tu'a" => Self::Tuha,
            "tu'ai" => Self::Tuhai,
            "tu'au" => Self::Tuhau,
            "tu'e" => Self::Tuhe,
            "tu'i" => Self::Tuhi,
            "tu'i'a" => Self::Tuhiha,
            "tu'i'e" => Self::Tuhihe,
            "tu'i'i" => Self::Tuhihi,
            "tu'i'o" => Self::Tuhiho,
            "tu'i'u" => Self::Tuhihu,
            "tu'o" => Self::Tuho,
            "tu'u" => Self::Tuhu,
            "ty" => Self::Ty,
            "u" => Self::U,
            "u'a" => Self::Uha,
            "u'e" => Self::Uhe,
            "u'i" => Self::Uhi,
            "u'o" => Self::Uho,
            "u'o'e" => Self::Uhohe,
            "u'o'i" => Self::Uhohi,
            "u'o'o" => Self::Uhoho,
            "u'o'u" => Self::Uhohu,
            "u'oi" => Self::Uhoi,
            "u'u" => Self::Uhu,
            "u'y" => Self::Uhy,
            "ua" => Self::Ua,
            "ue" => Self::Ue,
            "ue'i" => Self::Uehi,
            "ui" => Self::Ui,
            "ui'ai" => Self::Uihai,
            "uo" => Self::Uo,
            "uu" => Self::Uu,
            "uy" => Self::Uy,
            "va" => Self::Va,
            "va'a" => Self::Vaha,
            "va'e" => Self::Vahe,
            "va'ei" => Self::Vahei,
            "va'i" => Self::Vahi,
            "va'o" => Self::Vaho,
            "va'o'i" => Self::Vahohi,
            "va'u" => Self::Vahu,
            "vai" => Self::Vai,
            "vai'e" => Self::Vaihe,
            "vau" => Self::Vau,
            "ve" => Self::Ve,
            "ve'a" => Self::Veha,
            "ve'e" => Self::Vehe,
            "ve'i" => Self::Vehi,
            "ve'o" => Self::Veho,
            "ve'u" => Self::Vehu,
            "vei" => Self::Vei,
            "vi" => Self::Vi,
            "vi'a" => Self::Viha,
            "vi'e" => Self::Vihe,
            "vi'i" => Self::Vihi,
            "vi'o" => Self::Viho,
            "vi'u" => Self::Vihu,
            "vo" => Self::Vo,
            "vo'a" => Self::Voha,
            "vo'ai" => Self::Vohai,
            "vo'e" => Self::Vohe,
            "vo'i" => Self::Vohi,
            "vo'o" => Self::Voho,
            "vo'u" => Self::Vohu,
            "voi" => Self::Voi,
            "voi'e" => Self::Voihe,
            "voi'i" => Self::Voihi,
            "vu" => Self::Vu,
            "vu'a" => Self::Vuha,
            "vu'e" => Self::Vuhe,
            "vu'i" => Self::Vuhi,
            "vu'o" => Self::Vuho,
            "vu'u" => Self::Vuhu,
            "vy" => Self::Vy,
            "xa" => Self::Xa,
            "xa'o" => Self::Xaho,
            "xai" => Self::Xai,
            "xai'e" => Self::Xaihe,
            "xau'a" => Self::Xauha,
            "xau'e" => Self::Xauhe,
            "xau'i" => Self::Xauhi,
            "xau'o" => Self::Xauho,
            "xau'u" => Self::Xauhu,
            "xe" => Self::Xe,
            "xe'au" => Self::Xehau,
            "xe'e" => Self::Xehe,
            "xe'ei" => Self::Xehei,
            "xe'i'a" => Self::Xehiha,
            "xe'i'e" => Self::Xehihe,
            "xe'i'i" => Self::Xehihi,
            "xe'i'o" => Self::Xehiho,
            "xe'i'u" => Self::Xehihu,
            "xe'u" => Self::Xehu,
            "xei'e" => Self::Xeihe,
            "xi" => Self::Xi,
            "xi'e" => Self::Xihe,
            "xi'i" => Self::Xihi,
            "xo" => Self::Xo,
            "xo'ai" => Self::Xohai,
            "xo'e" => Self::Xohe,
            "xo'i" => Self::Xohi,
            "xo'o" => Self::Xoho,
            "xo'u" => Self::Xohu,
            "xoi" => Self::Xoi,
            "xoi'i" => Self::Xoihi,
            "xu" => Self::Xu,
            "xu'ai" => Self::Xuhai,
            "xu'au" => Self::Xuhau,
            "xu'ei" => Self::Xuhei,
            "xu'u" => Self::Xuhu,
            "xy" => Self::Xy,
            "y" => Self::Y,
            "y'y" => Self::Yhy,
            "za" => Self::Za,
            "za'a" => Self::Zaha,
            "za'ai" => Self::Zahai,
            "za'e" => Self::Zahe,
            "za'ei" => Self::Zahei,
            "za'i" => Self::Zahi,
            "za'o" => Self::Zaho,
            "za'o'a" => Self::Zahoha,
            "za'u" => Self::Zahu,
            "zai" => Self::Zai,
            "zau" => Self::Zau,
            "zau'a" => Self::Zauha,
            "zau'e" => Self::Zauhe,
            "zau'i" => Self::Zauhi,
            "zau'o" => Self::Zauho,
            "zau'u" => Self::Zauhu,
            "ze" => Self::Ze,
            "ze'a" => Self::Zeha,
            "ze'e" => Self::Zehe,
            "ze'i" => Self::Zehi,
            "ze'o" => Self::Zeho,
            "ze'oi" => Self::Zehoi,
            "ze'u" => Self::Zehu,
            "zei" => Self::Zei,
            "zi" => Self::Zi,
            "zi'e" => Self::Zihe,
            "zi'o" => Self::Ziho,
            "zo" => Self::Zo,
            "zo'a" => Self::Zoha,
            "zo'au" => Self::Zohau,
            "zo'e" => Self::Zohe,
            "zo'ei" => Self::Zohei,
            "zo'i" => Self::Zohi,
            "zo'o" => Self::Zoho,
            "zo'oi" => Self::Zohoi,
            "zo'u" => Self::Zohu,
            "zoi" => Self::Zoi,
            "zu" => Self::Zu,
            "zu'a" => Self::Zuha,
            "zu'ai" => Self::Zuhai,
            "zu'au" => Self::Zuhau,
            "zu'e" => Self::Zuhe,
            "zu'i" => Self::Zuhi,
            "zu'o" => Self::Zuho,
            "zu'u" => Self::Zuhu,
            "zy" => Self::Zy,
            _ => return None,
        })
    }

    #[requires(true)]
    #[bityzba::ensures(!ret.is_empty())]
    pub const fn canonical_text(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::Aha => "a'a",
            Self::Ahai => "a'ai",
            Self::Ahe => "a'e",
            Self::Ahi => "a'i",
            Self::Aho => "a'o",
            Self::Ahoi => "a'oi",
            Self::Ahu => "a'u",
            Self::Ahy => "a'y",
            Self::Ai => "ai",
            Self::Aihi => "ai'i",
            Self::Au => "au",
            Self::Auhau => "au'au",
            Self::Ba => "ba",
            Self::Baha => "ba'a",
            Self::Bahau => "ba'au",
            Self::Bahe => "ba'e",
            Self::Bahei => "ba'ei",
            Self::Bahi => "ba'i",
            Self::Baho => "ba'o",
            Self::Bahoi => "ba'oi",
            Self::Bahu => "ba'u",
            Self::Bai => "bai",
            Self::Baihau => "bai'au",
            Self::Bau => "bau",
            Self::Be => "be",
            Self::Beha => "be'a",
            Self::Behau => "be'au",
            Self::Behe => "be'e",
            Self::Behei => "be'ei",
            Self::Behi => "be'i",
            Self::Beho => "be'o",
            Self::Behu => "be'u",
            Self::Bei => "bei",
            Self::Bi => "bi",
            Self::Bihai => "bi'ai",
            Self::Bihe => "bi'e",
            Self::Bihi => "bi'i",
            Self::Biho => "bi'o",
            Self::Bihu => "bi'u",
            Self::Bo => "bo",
            Self::Bohai => "bo'ai",
            Self::Bohei => "bo'ei",
            Self::Boi => "boi",
            Self::Boihau => "boi'au",
            Self::Bu => "bu",
            Self::Buha => "bu'a",
            Self::Buhe => "bu'e",
            Self::Buhei => "bu'ei",
            Self::Buhi => "bu'i",
            Self::Buho => "bu'o",
            Self::Buhu => "bu'u",
            Self::Buhuhe => "bu'u'e",
            Self::By => "by",
            Self::Ca => "ca",
            Self::Caha => "ca'a",
            Self::Cahe => "ca'e",
            Self::Cahi => "ca'i",
            Self::Caho => "ca'o",
            Self::Cahu => "ca'u",
            Self::Cai => "cai",
            Self::Cau => "cau",
            Self::Cauhe => "cau'e",
            Self::Cauhi => "cau'i",
            Self::Ce => "ce",
            Self::Ceha => "ce'a",
            Self::Cehai => "ce'ai",
            Self::Cehe => "ce'e",
            Self::Cehi => "ce'i",
            Self::Ceho => "ce'o",
            Self::Cehu => "ce'u",
            Self::Cei => "cei",
            Self::Ceiha => "cei'a",
            Self::Ceihi => "cei'i",
            Self::Ci => "ci",
            Self::Cihe => "ci'e",
            Self::Cihi => "ci'i",
            Self::Ciho => "ci'o",
            Self::Cihu => "ci'u",
            Self::Co => "co",
            Self::Coha => "co'a",
            Self::Cohaha => "co'a'a",
            Self::Cohauha => "co'au'a",
            Self::Cohe => "co'e",
            Self::Cohi => "co'i",
            Self::Coho => "co'o",
            Self::Cohoi => "co'oi",
            Self::Cohu => "co'u",
            Self::Cohuha => "co'u'a",
            Self::Coi => "coi",
            Self::Cu => "cu",
            Self::Cuha => "cu'a",
            Self::Cuhe => "cu'e",
            Self::Cuhei => "cu'ei",
            Self::Cuhi => "cu'i",
            Self::Cuho => "cu'o",
            Self::Cuhu => "cu'u",
            Self::Cy => "cy",
            Self::Da => "da",
            Self::Daha => "da'a",
            Self::Dahe => "da'e",
            Self::Dahei => "da'ei",
            Self::Dahi => "da'i",
            Self::Daho => "da'o",
            Self::Dahoi => "da'oi",
            Self::Dahu => "da'u",
            Self::Dai => "dai",
            Self::Daiha => "dai'a",
            Self::Daihe => "dai'e",
            Self::Daihi => "dai'i",
            Self::Daiho => "dai'o",
            Self::Daihu => "dai'u",
            Self::Daihy => "dai'y",
            Self::Dau => "dau",
            Self::Dauha => "dau'a",
            Self::Dauhe => "dau'e",
            Self::Dauhi => "dau'i",
            Self::Dauho => "dau'o",
            Self::Dauhu => "dau'u",
            Self::De => "de",
            Self::Deha => "de'a",
            Self::Dehahu => "de'a'u",
            Self::Dehai => "de'ai",
            Self::Dehe => "de'e",
            Self::Dehei => "de'ei",
            Self::Dehi => "de'i",
            Self::Dehiha => "de'i'a",
            Self::Dehihe => "de'i'e",
            Self::Dehihi => "de'i'i",
            Self::Dehiho => "de'i'o",
            Self::Dehihu => "de'i'u",
            Self::Deho => "de'o",
            Self::Dehoha => "de'o'a",
            Self::Dehu => "de'u",
            Self::Dei => "dei",
            Self::Deiha => "dei'a",
            Self::Di => "di",
            Self::Digit0 => "0",
            Self::Digit1 => "1",
            Self::Digit2 => "2",
            Self::Digit3 => "3",
            Self::Digit4 => "4",
            Self::Digit5 => "5",
            Self::Digit6 => "6",
            Self::Digit7 => "7",
            Self::Digit8 => "8",
            Self::Digit9 => "9",
            Self::Diha => "di'a",
            Self::Dihai => "di'ai",
            Self::Dihe => "di'e",
            Self::Dihei => "di'ei",
            Self::Dihi => "di'i",
            Self::Diho => "di'o",
            Self::Dihu => "di'u",
            Self::Do => "do",
            Self::Doha => "do'a",
            Self::Dohai => "do'ai",
            Self::Dohe => "do'e",
            Self::Dohi => "do'i",
            Self::Doho => "do'o",
            Self::Dohu => "do'u",
            Self::Doi => "doi",
            Self::Du => "du",
            Self::Duha => "du'a",
            Self::Duhe => "du'e",
            Self::Duhei => "du'ei",
            Self::Duhi => "du'i",
            Self::Duho => "du'o",
            Self::Duhoi => "du'oi",
            Self::Duhu => "du'u",
            Self::Dy => "dy",
            Self::E => "e",
            Self::Eha => "e'a",
            Self::Ehe => "e'e",
            Self::Ehei => "e'ei",
            Self::Ehi => "e'i",
            Self::Eho => "e'o",
            Self::Ehu => "e'u",
            Self::Ehuhi => "e'u'i",
            Self::Ehy => "e'y",
            Self::Ei => "ei",
            Self::Eihai => "ei'ai",
            Self::Eihei => "ei'ei",
            Self::Fa => "fa",
            Self::Faha => "fa'a",
            Self::Fahai => "fa'ai",
            Self::Fahe => "fa'e",
            Self::Fahi => "fa'i",
            Self::Faho => "fa'o",
            Self::Fahu => "fa'u",
            Self::Fai => "fai",
            Self::Faihu => "fai'u",
            Self::Fau => "fau",
            Self::Fauha => "fau'a",
            Self::Fauhe => "fau'e",
            Self::Fauhi => "fau'i",
            Self::Fauho => "fau'o",
            Self::Fauhu => "fau'u",
            Self::Fe => "fe",
            Self::Feha => "fe'a",
            Self::Fehaha => "fe'a'a",
            Self::Fehahe => "fe'a'e",
            Self::Fehahi => "fe'a'i",
            Self::Fehaho => "fe'a'o",
            Self::Fehe => "fe'e",
            Self::Fehi => "fe'i",
            Self::Feho => "fe'o",
            Self::Fehu => "fe'u",
            Self::Fei => "fei",
            Self::Feihe => "fei'e",
            Self::Fi => "fi",
            Self::Fiha => "fi'a",
            Self::Fihau => "fi'au",
            Self::Fihe => "fi'e",
            Self::Fihi => "fi'i",
            Self::Fiho => "fi'o",
            Self::Fihoi => "fi'oi",
            Self::Fihu => "fi'u",
            Self::Fo => "fo",
            Self::Foha => "fo'a",
            Self::Fohai => "fo'ai",
            Self::Fohe => "fo'e",
            Self::Fohi => "fo'i",
            Self::Foho => "fo'o",
            Self::Fohu => "fo'u",
            Self::Foi => "foi",
            Self::Fu => "fu",
            Self::Fuha => "fu'a",
            Self::Fuhau => "fu'au",
            Self::Fuhe => "fu'e",
            Self::Fuhi => "fu'i",
            Self::Fuho => "fu'o",
            Self::Fuhu => "fu'u",
            Self::Fy => "fy",
            Self::Ga => "ga",
            Self::Gaha => "ga'a",
            Self::Gahe => "ga'e",
            Self::Gahei => "ga'ei",
            Self::Gahi => "ga'i",
            Self::Gahihi => "ga'i'i",
            Self::Gaho => "ga'o",
            Self::Gahu => "ga'u",
            Self::Gahuhi => "ga'u'i",
            Self::Gai => "gai",
            Self::Gaiha => "gai'a",
            Self::Gaihe => "gai'e",
            Self::Gaihi => "gai'i",
            Self::Gaiho => "gai'o",
            Self::Gaihu => "gai'u",
            Self::Gau => "gau",
            Self::Gauhi => "gau'i",
            Self::Ge => "ge",
            Self::Geha => "ge'a",
            Self::Gehai => "ge'ai",
            Self::Gehe => "ge'e",
            Self::Gehi => "ge'i",
            Self::Geho => "ge'o",
            Self::Gehu => "ge'u",
            Self::Gehuhi => "ge'u'i",
            Self::Gei => "gei",
            Self::Geiha => "gei'a",
            Self::Gi => "gi",
            Self::Giha => "gi'a",
            Self::Gihe => "gi'e",
            Self::Gihi => "gi'i",
            Self::Giho => "gi'o",
            Self::Gihu => "gi'u",
            Self::Go => "go",
            Self::Goha => "go'a",
            Self::Gohe => "go'e",
            Self::Gohi => "go'i",
            Self::Goho => "go'o",
            Self::Gohoi => "go'oi",
            Self::Gohu => "go'u",
            Self::Goi => "goi",
            Self::Gu => "gu",
            Self::Guha => "gu'a",
            Self::Guhe => "gu'e",
            Self::Guhi => "gu'i",
            Self::Guho => "gu'o",
            Self::Guhu => "gu'u",
            Self::Gy => "gy",
            Self::I => "i",
            Self::Ia => "ia",
            Self::Iahau => "ia'au",
            Self::Iahu => "ia'u",
            Self::Ie => "ie",
            Self::Ieho => "ie'o",
            Self::Iha => "i'a",
            Self::Ihau => "i'au",
            Self::Ihe => "i'e",
            Self::Ihei => "i'ei",
            Self::Ihi => "i'i",
            Self::Ihihi => "i'i'i",
            Self::Iho => "i'o",
            Self::Ihu => "i'u",
            Self::Ihy => "i'y",
            Self::Ii => "ii",
            Self::Io => "io",
            Self::Iu => "iu",
            Self::Iy => "iy",
            Self::Ja => "ja",
            Self::Jaha => "ja'a",
            Self::Jahai => "ja'ai",
            Self::Jahau => "ja'au",
            Self::Jahe => "ja'e",
            Self::Jahei => "ja'ei",
            Self::Jahi => "ja'i",
            Self::Jaho => "ja'o",
            Self::Jahohe => "ja'o'e",
            Self::Jahoho => "ja'o'o",
            Self::Jahoi => "ja'oi",
            Self::Jahui => "ja'ui",
            Self::Jai => "jai",
            Self::Jau => "jau",
            Self::Jauha => "jau'a",
            Self::Jauhe => "jau'e",
            Self::Jauhi => "jau'i",
            Self::Jauho => "jau'o",
            Self::Jauhu => "jau'u",
            Self::Je => "je",
            Self::Jeha => "je'a",
            Self::Jehau => "je'au",
            Self::Jehe => "je'e",
            Self::Jehi => "je'i",
            Self::Jeho => "je'o",
            Self::Jehu => "je'u",
            Self::Jei => "jei",
            Self::Jeihe => "jei'e",
            Self::Jeihi => "jei'i",
            Self::Jeiho => "jei'o",
            Self::Ji => "ji",
            Self::Jiha => "ji'a",
            Self::Jihai => "ji'ai",
            Self::Jihe => "ji'e",
            Self::Jihehe => "ji'e'e",
            Self::Jihei => "ji'ei",
            Self::Jihi => "ji'i",
            Self::Jihiha => "ji'i'a",
            Self::Jiho => "ji'o",
            Self::Jihohe => "ji'o'e",
            Self::Jihoho => "ji'o'o",
            Self::Jihu => "ji'u",
            Self::Jo => "jo",
            Self::Joha => "jo'a",
            Self::Johai => "jo'ai",
            Self::Johau => "jo'au",
            Self::Johe => "jo'e",
            Self::Johi => "jo'i",
            Self::Johiha => "jo'i'a",
            Self::Joho => "jo'o",
            Self::Johu => "jo'u",
            Self::Johuhu => "jo'u'u",
            Self::Joi => "joi",
            Self::Joihe => "joi'e",
            Self::Joihi => "joi'i",
            Self::Joiho => "joi'o",
            Self::Joihu => "joi'u",
            Self::Ju => "ju",
            Self::Juha => "ju'a",
            Self::Juhe => "ju'e",
            Self::Juhi => "ju'i",
            Self::Juho => "ju'o",
            Self::Juhoi => "ju'oi",
            Self::Juhu => "ju'u",
            Self::Jy => "jy",
            Self::Ka => "ka",
            Self::Kaha => "ka'a",
            Self::Kahai => "ka'ai",
            Self::Kahe => "ka'e",
            Self::Kahi => "ka'i",
            Self::Kaho => "ka'o",
            Self::Kahu => "ka'u",
            Self::Kai => "kai",
            Self::Kaihai => "kai'ai",
            Self::Kaihu => "kai'u",
            Self::Kau => "kau",
            Self::Kauha => "kau'a",
            Self::Kauhe => "kau'e",
            Self::Kauhi => "kau'i",
            Self::Kauho => "kau'o",
            Self::Kauhu => "kau'u",
            Self::Ke => "ke",
            Self::Keha => "ke'a",
            Self::Kehau => "ke'au",
            Self::Kehe => "ke'e",
            Self::Kehi => "ke'i",
            Self::Kehihai => "ke'i'ai",
            Self::Keho => "ke'o",
            Self::Kehu => "ke'u",
            Self::Kei => "kei",
            Self::Ki => "ki",
            Self::Kiha => "ki'a",
            Self::Kihaha => "ki'a'a",
            Self::Kihai => "ki'ai",
            Self::Kihe => "ki'e",
            Self::Kiheha => "ki'e'a",
            Self::Kihi => "ki'i",
            Self::Kihiha => "ki'i'a",
            Self::Kiho => "ki'o",
            Self::Kihoha => "ki'o'a",
            Self::Kihohe => "ki'o'e",
            Self::Kihoi => "ki'oi",
            Self::Kihu => "ki'u",
            Self::Kihuha => "ki'u'a",
            Self::Kihuhe => "ki'u'e",
            Self::Kihuhi => "ki'u'i",
            Self::Ko => "ko",
            Self::Koha => "ko'a",
            Self::Kohau => "ko'au",
            Self::Kohe => "ko'e",
            Self::Kohi => "ko'i",
            Self::Koho => "ko'o",
            Self::Kohoi => "ko'oi",
            Self::Kohu => "ko'u",
            Self::Koi => "koi",
            Self::Ku => "ku",
            Self::Kuha => "ku'a",
            Self::Kuhau => "ku'au",
            Self::Kuhe => "ku'e",
            Self::Kuhi => "ku'i",
            Self::Kuho => "ku'o",
            Self::Kuhoi => "ku'oi",
            Self::Kuhu => "ku'u",
            Self::Ky => "ky",
            Self::La => "la",
            Self::Laha => "la'a",
            Self::Lahai => "la'ai",
            Self::Lahau => "la'au",
            Self::Lahe => "la'e",
            Self::Lahei => "la'ei",
            Self::Lahi => "la'i",
            Self::Laho => "la'o",
            Self::Lahoho => "la'o'o",
            Self::Lahoi => "la'oi",
            Self::Lahu => "la'u",
            Self::Lai => "lai",
            Self::Lau => "lau",
            Self::Le => "le",
            Self::Leha => "le'a",
            Self::Lehai => "le'ai",
            Self::Lehe => "le'e",
            Self::Lehei => "le'ei",
            Self::Lehi => "le'i",
            Self::Leho => "le'o",
            Self::Lehohe => "le'o'e",
            Self::Lehu => "le'u",
            Self::Lei => "lei",
            Self::Leihe => "lei'e",
            Self::Leihi => "lei'i",
            Self::Li => "li",
            Self::Liha => "li'a",
            Self::Lihai => "li'ai",
            Self::Lihau => "li'au",
            Self::Lihe => "li'e",
            Self::Lihehe => "li'e'e",
            Self::Lihei => "li'ei",
            Self::Lihi => "li'i",
            Self::Liho => "li'o",
            Self::Lihoi => "li'oi",
            Self::Lihu => "li'u",
            Self::Lo => "lo",
            Self::Loha => "lo'a",
            Self::Lohai => "lo'ai",
            Self::Lohe => "lo'e",
            Self::Lohei => "lo'ei",
            Self::Lohi => "lo'i",
            Self::Loho => "lo'o",
            Self::Lohoi => "lo'oi",
            Self::Lohu => "lo'u",
            Self::Loi => "loi",
            Self::Loihe => "loi'e",
            Self::Loihi => "loi'i",
            Self::Lu => "lu",
            Self::Luha => "lu'a",
            Self::Luhe => "lu'e",
            Self::Luhei => "lu'ei",
            Self::Luhi => "lu'i",
            Self::Luho => "lu'o",
            Self::Luhu => "lu'u",
            Self::Ly => "ly",
            Self::Ma => "ma",
            Self::Maha => "ma'a",
            Self::Mahai => "ma'ai",
            Self::Mahau => "ma'au",
            Self::Mahe => "ma'e",
            Self::Mahei => "ma'ei",
            Self::Mahi => "ma'i",
            Self::Maho => "ma'o",
            Self::Mahoi => "ma'oi",
            Self::Mahu => "ma'u",
            Self::Mai => "mai",
            Self::Maiho => "mai'o",
            Self::Mau => "mau",
            Self::Mauha => "mau'a",
            Self::Mauhe => "mau'e",
            Self::Mauhi => "mau'i",
            Self::Mauho => "mau'o",
            Self::Mauhu => "mau'u",
            Self::Me => "me",
            Self::Meha => "me'a",
            Self::Mehau => "me'au",
            Self::Mehe => "me'e",
            Self::Mehei => "me'ei",
            Self::Mehi => "me'i",
            Self::Meho => "me'o",
            Self::Mehohe => "me'o'e",
            Self::Mehoi => "me'oi",
            Self::Mehu => "me'u",
            Self::Mei => "mei",
            Self::Mi => "mi",
            Self::Miha => "mi'a",
            Self::Mihai => "mi'ai",
            Self::Mihau => "mi'au",
            Self::Mihe => "mi'e",
            Self::Mihei => "mi'ei",
            Self::Mihi => "mi'i",
            Self::Miho => "mi'o",
            Self::Mihu => "mi'u",
            Self::Mo => "mo",
            Self::Moha => "mo'a",
            Self::Mohe => "mo'e",
            Self::Mohi => "mo'i",
            Self::Moho => "mo'o",
            Self::Mohoi => "mo'oi",
            Self::Mohu => "mo'u",
            Self::Moi => "moi",
            Self::Moiho => "moi'o",
            Self::Moihoi => "moi'oi",
            Self::Mu => "mu",
            Self::Muha => "mu'a",
            Self::Muhai => "mu'ai",
            Self::Muhe => "mu'e",
            Self::Muhei => "mu'ei",
            Self::Muhi => "mu'i",
            Self::Muho => "mu'o",
            Self::Muhoi => "mu'oi",
            Self::Muhu => "mu'u",
            Self::My => "my",
            Self::Na => "na",
            Self::Naha => "na'a",
            Self::Nahe => "na'e",
            Self::Nahei => "na'ei",
            Self::Nahi => "na'i",
            Self::Naho => "na'o",
            Self::Nahoi => "na'oi",
            Self::Nahu => "na'u",
            Self::Nai => "nai",
            Self::Nau => "nau",
            Self::Nauho => "nau'o",
            Self::Nauhu => "nau'u",
            Self::Ne => "ne",
            Self::Neha => "ne'a",
            Self::Nehahi => "ne'a'i",
            Self::Nehi => "ne'i",
            Self::Neho => "ne'o",
            Self::Nehu => "ne'u",
            Self::Nei => "nei",
            Self::Ni => "ni",
            Self::Niha => "ni'a",
            Self::Nihe => "ni'e",
            Self::Nihei => "ni'ei",
            Self::Nihi => "ni'i",
            Self::Nihihi => "ni'i'i",
            Self::Niho => "ni'o",
            Self::Nihu => "ni'u",
            Self::No => "no",
            Self::Noha => "no'a",
            Self::Nohe => "no'e",
            Self::Nohei => "no'ei",
            Self::Nohi => "no'i",
            Self::Noho => "no'o",
            Self::Nohoi => "no'oi",
            Self::Nohu => "no'u",
            Self::Noi => "noi",
            Self::Noiha => "noi'a",
            Self::Noihi => "noi'i",
            Self::Noihoha => "noi'o'a",
            Self::Nu => "nu",
            Self::Nuha => "nu'a",
            Self::Nuhe => "nu'e",
            Self::Nuhi => "nu'i",
            Self::Nuho => "nu'o",
            Self::Nuhu => "nu'u",
            Self::Ny => "ny",
            Self::O => "o",
            Self::Oha => "o'a",
            Self::Ohai => "o'ai",
            Self::Ohe => "o'e",
            Self::Ohi => "o'i",
            Self::Oho => "o'o",
            Self::Ohu => "o'u",
            Self::Ohy => "o'y",
            Self::Oi => "oi",
            Self::Oiha => "oi'a",
            Self::Oihoi => "oi'oi",
            Self::Pa => "pa",
            Self::Paha => "pa'a",
            Self::Pahahi => "pa'a'i",
            Self::Pahe => "pa'e",
            Self::Pahi => "pa'i",
            Self::Paho => "pa'o",
            Self::Pahu => "pa'u",
            Self::Pai => "pai",
            Self::Paihe => "pai'e",
            Self::Pau => "pau",
            Self::Pe => "pe",
            Self::Peha => "pe'a",
            Self::Pehahi => "pe'a'i",
            Self::Pehe => "pe'e",
            Self::Pehei => "pe'ei",
            Self::Pehi => "pe'i",
            Self::Peho => "pe'o",
            Self::Pehu => "pe'u",
            Self::Pei => "pei",
            Self::Peihe => "pei'e",
            Self::Pi => "pi",
            Self::Piha => "pi'a",
            Self::Pihai => "pi'ai",
            Self::Pihe => "pi'e",
            Self::Pihei => "pi'ei",
            Self::Pihi => "pi'i",
            Self::Piho => "pi'o",
            Self::Pihu => "pi'u",
            Self::Po => "po",
            Self::Pohai => "po'ai",
            Self::Pohe => "po'e",
            Self::Pohi => "po'i",
            Self::Poho => "po'o",
            Self::Pohoi => "po'oi",
            Self::Pohu => "po'u",
            Self::Poi => "poi",
            Self::Poiha => "poi'a",
            Self::Poihei => "poi'ei",
            Self::Poihi => "poi'i",
            Self::Poihoha => "poi'o'a",
            Self::Pu => "pu",
            Self::Puha => "pu'a",
            Self::Puhau => "pu'au",
            Self::Puhe => "pu'e",
            Self::Puhehi => "pu'e'i",
            Self::Puhi => "pu'i",
            Self::Puhiha => "pu'i'a",
            Self::Puhihi => "pu'i'i",
            Self::Puho => "pu'o",
            Self::Puhohi => "pu'o'i",
            Self::Puhu => "pu'u",
            Self::Py => "py",
            Self::Ra => "ra",
            Self::Raha => "ra'a",
            Self::Rahai => "ra'ai",
            Self::Rahe => "ra'e",
            Self::Rahi => "ra'i",
            Self::Raho => "ra'o",
            Self::Rahoi => "ra'oi",
            Self::Rahu => "ra'u",
            Self::Rai => "rai",
            Self::Raihe => "rai'e",
            Self::Rau => "rau",
            Self::Rauhi => "rau'i",
            Self::Re => "re",
            Self::Reha => "re'a",
            Self::Rehe => "re'e",
            Self::Rehei => "re'ei",
            Self::Rehi => "re'i",
            Self::Reho => "re'o",
            Self::Rehu => "re'u",
            Self::Rei => "rei",
            Self::Ri => "ri",
            Self::Riha => "ri'a",
            Self::Rihe => "ri'e",
            Self::Rihi => "ri'i",
            Self::Rihiha => "ri'i'a",
            Self::Rihihe => "ri'i'e",
            Self::Rihihi => "ri'i'i",
            Self::Rihiho => "ri'i'o",
            Self::Rihihu => "ri'i'u",
            Self::Riho => "ri'o",
            Self::Rihoi => "ri'oi",
            Self::Rihu => "ri'u",
            Self::Ro => "ro",
            Self::Roha => "ro'a",
            Self::Rohe => "ro'e",
            Self::Rohei => "ro'ei",
            Self::Rohi => "ro'i",
            Self::Roho => "ro'o",
            Self::Rohoi => "ro'oi",
            Self::Rohu => "ro'u",
            Self::Roi => "roi",
            Self::Ru => "ru",
            Self::Ruha => "ru'a",
            Self::Ruhe => "ru'e",
            Self::Ruhi => "ru'i",
            Self::Ruho => "ru'o",
            Self::Ruhu => "ru'u",
            Self::Ry => "ry",
            Self::Sa => "sa",
            Self::Saha => "sa'a",
            Self::Sahai => "sa'ai",
            Self::Sahe => "sa'e",
            Self::Sahei => "sa'ei",
            Self::Sahi => "sa'i",
            Self::Sahiha => "sa'i'a",
            Self::Saho => "sa'o",
            Self::Sahu => "sa'u",
            Self::Sai => "sai",
            Self::Saihe => "sai'e",
            Self::Saihi => "sai'i",
            Self::Sau => "sau",
            Self::Sauha => "sau'a",
            Self::Se => "se",
            Self::Seha => "se'a",
            Self::Sehe => "se'e",
            Self::Sehi => "se'i",
            Self::Seho => "se'o",
            Self::Sehu => "se'u",
            Self::Sei => "sei",
            Self::Seiha => "sei'a",
            Self::Seihe => "sei'e",
            Self::Seihi => "sei'i",
            Self::Si => "si",
            Self::Siha => "si'a",
            Self::Sihau => "si'au",
            Self::Sihe => "si'e",
            Self::Sihi => "si'i",
            Self::Siho => "si'o",
            Self::Sihu => "si'u",
            Self::So => "so",
            Self::Soha => "so'a",
            Self::Sohahu => "so'a'u",
            Self::Sohai => "so'ai",
            Self::Sohe => "so'e",
            Self::Sohei => "so'ei",
            Self::Sohi => "so'i",
            Self::Soho => "so'o",
            Self::Sohoi => "so'oi",
            Self::Sohu => "so'u",
            Self::Soi => "soi",
            Self::Soiha => "soi'a",
            Self::Soihe => "soi'e",
            Self::Su => "su",
            Self::Suha => "su'a",
            Self::Suhai => "su'ai",
            Self::Suhe => "su'e",
            Self::Suhei => "su'ei",
            Self::Suhi => "su'i",
            Self::Suho => "su'o",
            Self::Suhoi => "su'oi",
            Self::Suhu => "su'u",
            Self::Sy => "sy",
            Self::Ta => "ta",
            Self::Taha => "ta'a",
            Self::Tahai => "ta'ai",
            Self::Tahe => "ta'e",
            Self::Tahi => "ta'i",
            Self::Tahiha => "ta'i'a",
            Self::Tahihe => "ta'i'e",
            Self::Tahihi => "ta'i'i",
            Self::Tahiho => "ta'i'o",
            Self::Tahihu => "ta'i'u",
            Self::Taho => "ta'o",
            Self::Tahu => "ta'u",
            Self::Tahuhi => "ta'u'i",
            Self::Tai => "tai",
            Self::Tau => "tau",
            Self::Te => "te",
            Self::Teha => "te'a",
            Self::Tehai => "te'ai",
            Self::Tehe => "te'e",
            Self::Teho => "te'o",
            Self::Tehoi => "te'oi",
            Self::Tehu => "te'u",
            Self::Tei => "tei",
            Self::Ti => "ti",
            Self::Tiha => "ti'a",
            Self::Tihau => "ti'au",
            Self::Tihe => "ti'e",
            Self::Tihi => "ti'i",
            Self::Tihiha => "ti'i'a",
            Self::Tiho => "ti'o",
            Self::Tihu => "ti'u",
            Self::Tihuha => "ti'u'a",
            Self::Tihuhi => "ti'u'i",
            Self::Tihuhu => "ti'u'u",
            Self::To => "to",
            Self::Toha => "to'a",
            Self::Tohai => "to'ai",
            Self::Tohe => "to'e",
            Self::Tohi => "to'i",
            Self::Toho => "to'o",
            Self::Tohohe => "to'o'e",
            Self::Tohu => "to'u",
            Self::Toi => "toi",
            Self::Tu => "tu",
            Self::Tuha => "tu'a",
            Self::Tuhai => "tu'ai",
            Self::Tuhau => "tu'au",
            Self::Tuhe => "tu'e",
            Self::Tuhi => "tu'i",
            Self::Tuhiha => "tu'i'a",
            Self::Tuhihe => "tu'i'e",
            Self::Tuhihi => "tu'i'i",
            Self::Tuhiho => "tu'i'o",
            Self::Tuhihu => "tu'i'u",
            Self::Tuho => "tu'o",
            Self::Tuhu => "tu'u",
            Self::Ty => "ty",
            Self::U => "u",
            Self::Ua => "ua",
            Self::Ue => "ue",
            Self::Uehi => "ue'i",
            Self::Uha => "u'a",
            Self::Uhe => "u'e",
            Self::Uhi => "u'i",
            Self::Uho => "u'o",
            Self::Uhohe => "u'o'e",
            Self::Uhohi => "u'o'i",
            Self::Uhoho => "u'o'o",
            Self::Uhohu => "u'o'u",
            Self::Uhoi => "u'oi",
            Self::Uhu => "u'u",
            Self::Uhy => "u'y",
            Self::Ui => "ui",
            Self::Uihai => "ui'ai",
            Self::Uo => "uo",
            Self::Uu => "uu",
            Self::Uy => "uy",
            Self::Va => "va",
            Self::Vaha => "va'a",
            Self::Vahe => "va'e",
            Self::Vahei => "va'ei",
            Self::Vahi => "va'i",
            Self::Vaho => "va'o",
            Self::Vahohi => "va'o'i",
            Self::Vahu => "va'u",
            Self::Vai => "vai",
            Self::Vaihe => "vai'e",
            Self::Vau => "vau",
            Self::Ve => "ve",
            Self::Veha => "ve'a",
            Self::Vehe => "ve'e",
            Self::Vehi => "ve'i",
            Self::Veho => "ve'o",
            Self::Vehu => "ve'u",
            Self::Vei => "vei",
            Self::Vi => "vi",
            Self::Viha => "vi'a",
            Self::Vihe => "vi'e",
            Self::Vihi => "vi'i",
            Self::Viho => "vi'o",
            Self::Vihu => "vi'u",
            Self::Vo => "vo",
            Self::Voha => "vo'a",
            Self::Vohai => "vo'ai",
            Self::Vohe => "vo'e",
            Self::Vohi => "vo'i",
            Self::Voho => "vo'o",
            Self::Vohu => "vo'u",
            Self::Voi => "voi",
            Self::Voihe => "voi'e",
            Self::Voihi => "voi'i",
            Self::Vu => "vu",
            Self::Vuha => "vu'a",
            Self::Vuhe => "vu'e",
            Self::Vuhi => "vu'i",
            Self::Vuho => "vu'o",
            Self::Vuhu => "vu'u",
            Self::Vy => "vy",
            Self::Xa => "xa",
            Self::Xaho => "xa'o",
            Self::Xai => "xai",
            Self::Xaihe => "xai'e",
            Self::Xauha => "xau'a",
            Self::Xauhe => "xau'e",
            Self::Xauhi => "xau'i",
            Self::Xauho => "xau'o",
            Self::Xauhu => "xau'u",
            Self::Xe => "xe",
            Self::Xehau => "xe'au",
            Self::Xehe => "xe'e",
            Self::Xehei => "xe'ei",
            Self::Xehiha => "xe'i'a",
            Self::Xehihe => "xe'i'e",
            Self::Xehihi => "xe'i'i",
            Self::Xehiho => "xe'i'o",
            Self::Xehihu => "xe'i'u",
            Self::Xehu => "xe'u",
            Self::Xeihe => "xei'e",
            Self::Xi => "xi",
            Self::Xihe => "xi'e",
            Self::Xihi => "xi'i",
            Self::Xo => "xo",
            Self::Xohai => "xo'ai",
            Self::Xohe => "xo'e",
            Self::Xohi => "xo'i",
            Self::Xoho => "xo'o",
            Self::Xohu => "xo'u",
            Self::Xoi => "xoi",
            Self::Xoihi => "xoi'i",
            Self::Xu => "xu",
            Self::Xuhai => "xu'ai",
            Self::Xuhau => "xu'au",
            Self::Xuhei => "xu'ei",
            Self::Xuhu => "xu'u",
            Self::Xy => "xy",
            Self::Y => "y",
            Self::Yhy => "y'y",
            Self::Za => "za",
            Self::Zaha => "za'a",
            Self::Zahai => "za'ai",
            Self::Zahe => "za'e",
            Self::Zahei => "za'ei",
            Self::Zahi => "za'i",
            Self::Zaho => "za'o",
            Self::Zahoha => "za'o'a",
            Self::Zahu => "za'u",
            Self::Zai => "zai",
            Self::Zau => "zau",
            Self::Zauha => "zau'a",
            Self::Zauhe => "zau'e",
            Self::Zauhi => "zau'i",
            Self::Zauho => "zau'o",
            Self::Zauhu => "zau'u",
            Self::Ze => "ze",
            Self::Zeha => "ze'a",
            Self::Zehe => "ze'e",
            Self::Zehi => "ze'i",
            Self::Zeho => "ze'o",
            Self::Zehoi => "ze'oi",
            Self::Zehu => "ze'u",
            Self::Zei => "zei",
            Self::Zi => "zi",
            Self::Zihe => "zi'e",
            Self::Ziho => "zi'o",
            Self::Zo => "zo",
            Self::Zoha => "zo'a",
            Self::Zohau => "zo'au",
            Self::Zohe => "zo'e",
            Self::Zohei => "zo'ei",
            Self::Zohi => "zo'i",
            Self::Zoho => "zo'o",
            Self::Zohoi => "zo'oi",
            Self::Zohu => "zo'u",
            Self::Zoi => "zoi",
            Self::Zu => "zu",
            Self::Zuha => "zu'a",
            Self::Zuhai => "zu'ai",
            Self::Zuhau => "zu'au",
            Self::Zuhe => "zu'e",
            Self::Zuhi => "zu'i",
            Self::Zuho => "zu'o",
            Self::Zuhu => "zu'u",
            Self::Zy => "zy",
        }
    }

    #[requires(true)]
    #[bityzba::ensures(true)]
    pub fn is_selmaho(self, selmaho: Selmaho) -> bool {
        selmaho.contains(self)
    }
}

impl fmt::Display for Cmavo {
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_text())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Selmaho {
    A,
    Bahe,
    Bai,
    Be,
    Beho,
    Bei,
    Bihi,
    Bu,
    By,
    Caha,
    Cai,
    Cehe,
    Co,
    Coi,
    Cu,
    Cuhe,
    Daho,
    Doi,
    Fa,
    Faha,
    Faho,
    Fuha,
    Ga,
    Gaho,
    Gi,
    Giha,
    Gihi,
    Goha,
    Goi,
    Guha,
    I,
    Ja,
    Jai,
    Jehi,
    Johi,
    Joi,
    Koha,
    Ku,
    La,
    Lahe,
    Lau,
    Le,
    Lehu,
    Li,
    Lihau,
    Lihu,
    Loho,
    Lohoi,
    Lohu,
    Lu,
    Luhei,
    Mai,
    Me,
    Mohe,
    Mohi,
    Moi,
    Na,
    Nahe,
    Nai,
    Niho,
    Noi,
    Noiha,
    Nu,
    Pa,
    Pehe,
    Pu,
    Roi,
    Sa,
    Se,
    Sehu,
    Sei,
    Si,
    Soi,
    Su,
    Tahe,
    To,
    Toi,
    Tuhe,
    Ui,
    Ui3a,
    Va,
    Vau,
    Veha,
    Veho,
    Vei,
    Viha,
    Vuhu,
    Xi,
    Y,
    Zaho,
    Zeha,
    Zei,
    Zi,
    Zo,
    Zohu,
    Zoi,
}

impl Selmaho {
    #[requires(true)]
    #[bityzba::ensures(!ret.is_empty())]
    pub const fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Bahe => "BAhE",
            Self::Bai => "BAI",
            Self::Be => "BE",
            Self::Beho => "BEhO",
            Self::Bei => "BEI",
            Self::Bihi => "BIhI",
            Self::Bu => "BU",
            Self::By => "BY",
            Self::Caha => "CAhA",
            Self::Cai => "CAI",
            Self::Cehe => "CEhE",
            Self::Co => "CO",
            Self::Coi => "COI",
            Self::Cu => "CU",
            Self::Cuhe => "CUhE",
            Self::Daho => "DAhO",
            Self::Doi => "DOI",
            Self::Fa => "FA",
            Self::Faha => "FAhA",
            Self::Faho => "FAhO",
            Self::Fuha => "FUhA",
            Self::Ga => "GA",
            Self::Gaho => "GAhO",
            Self::Gi => "GI",
            Self::Giha => "GIhA",
            Self::Gihi => "GIhI",
            Self::Goha => "GOhA",
            Self::Goi => "GOI",
            Self::Guha => "GUhA",
            Self::I => "I",
            Self::Ja => "JA",
            Self::Jai => "JAI",
            Self::Jehi => "JEhI",
            Self::Johi => "JOhI",
            Self::Joi => "JOI",
            Self::Koha => "KOhA",
            Self::Ku => "KU",
            Self::La => "LA",
            Self::Lahe => "LAhE",
            Self::Lau => "LAU",
            Self::Le => "LE",
            Self::Lehu => "LEhU",
            Self::Li => "LI",
            Self::Lihau => "LIhAU",
            Self::Lihu => "LIhU",
            Self::Loho => "LOhO",
            Self::Lohoi => "LOhOI",
            Self::Lohu => "LOhU",
            Self::Lu => "LU",
            Self::Luhei => "LUhEI",
            Self::Mai => "MAI",
            Self::Me => "ME",
            Self::Mohe => "MOhE",
            Self::Mohi => "MOhI",
            Self::Moi => "MOI",
            Self::Na => "NA",
            Self::Nahe => "NAhE",
            Self::Nai => "NAI",
            Self::Niho => "NIhO",
            Self::Noi => "NOI",
            Self::Noiha => "NOIhA",
            Self::Nu => "NU",
            Self::Pa => "PA",
            Self::Pehe => "PEhE",
            Self::Pu => "PU",
            Self::Roi => "ROI",
            Self::Sa => "SA",
            Self::Se => "SE",
            Self::Sehu => "SEhU",
            Self::Sei => "SEI",
            Self::Si => "SI",
            Self::Soi => "SOI",
            Self::Su => "SU",
            Self::Tahe => "TAhE",
            Self::To => "TO",
            Self::Toi => "TOI",
            Self::Tuhe => "TUhE",
            Self::Ui => "UI",
            Self::Ui3a => "UI3a",
            Self::Va => "VA",
            Self::Vau => "VAU",
            Self::Veha => "VEhA",
            Self::Veho => "VEhO",
            Self::Vei => "VEI",
            Self::Viha => "VIhA",
            Self::Vuhu => "VUhU",
            Self::Xi => "XI",
            Self::Y => "Y",
            Self::Zaho => "ZAhO",
            Self::Zeha => "ZEhA",
            Self::Zei => "ZEI",
            Self::Zi => "ZI",
            Self::Zo => "ZO",
            Self::Zohu => "ZOhU",
            Self::Zoi => "ZOI",
        }
    }

    #[requires(!name.is_empty())]
    #[bityzba::ensures(ret.is_none() || ret.unwrap().name() == name)]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "A" => Self::A,
            "BAhE" => Self::Bahe,
            "BAI" => Self::Bai,
            "BE" => Self::Be,
            "BEhO" => Self::Beho,
            "BEI" => Self::Bei,
            "BIhI" => Self::Bihi,
            "BU" => Self::Bu,
            "BY" => Self::By,
            "CAhA" => Self::Caha,
            "CAI" => Self::Cai,
            "CEhE" => Self::Cehe,
            "CO" => Self::Co,
            "COI" => Self::Coi,
            "CU" => Self::Cu,
            "CUhE" => Self::Cuhe,
            "DAhO" => Self::Daho,
            "DOI" => Self::Doi,
            "FA" => Self::Fa,
            "FAhA" => Self::Faha,
            "FAhO" => Self::Faho,
            "FUhA" => Self::Fuha,
            "GA" => Self::Ga,
            "GAhO" => Self::Gaho,
            "GI" => Self::Gi,
            "GIhA" => Self::Giha,
            "GIhI" => Self::Gihi,
            "GOhA" => Self::Goha,
            "GOI" => Self::Goi,
            "GUhA" => Self::Guha,
            "I" => Self::I,
            "JA" => Self::Ja,
            "JAI" => Self::Jai,
            "JEhI" => Self::Jehi,
            "JOhI" => Self::Johi,
            "JOI" => Self::Joi,
            "KOhA" => Self::Koha,
            "KU" => Self::Ku,
            "LA" => Self::La,
            "LAhE" => Self::Lahe,
            "LAU" => Self::Lau,
            "LE" => Self::Le,
            "LEhU" => Self::Lehu,
            "LI" => Self::Li,
            "LIhAU" => Self::Lihau,
            "LIhU" => Self::Lihu,
            "LOhO" => Self::Loho,
            "LOhOI" => Self::Lohoi,
            "LOhU" => Self::Lohu,
            "LU" => Self::Lu,
            "LUhEI" => Self::Luhei,
            "MAI" => Self::Mai,
            "ME" => Self::Me,
            "MOhE" => Self::Mohe,
            "MOhI" => Self::Mohi,
            "MOI" => Self::Moi,
            "NA" => Self::Na,
            "NAhE" => Self::Nahe,
            "NAI" => Self::Nai,
            "NIhO" => Self::Niho,
            "NOI" => Self::Noi,
            "NOIhA" => Self::Noiha,
            "NU" => Self::Nu,
            "PA" => Self::Pa,
            "PEhE" => Self::Pehe,
            "PU" => Self::Pu,
            "ROI" => Self::Roi,
            "SA" => Self::Sa,
            "SE" => Self::Se,
            "SEhU" => Self::Sehu,
            "SEI" => Self::Sei,
            "SI" => Self::Si,
            "SOI" => Self::Soi,
            "SU" => Self::Su,
            "TAhE" => Self::Tahe,
            "TO" => Self::To,
            "TOI" => Self::Toi,
            "TUhE" => Self::Tuhe,
            "UI" => Self::Ui,
            "UI3a" => Self::Ui3a,
            "VA" => Self::Va,
            "VAU" => Self::Vau,
            "VEhA" => Self::Veha,
            "VEhO" => Self::Veho,
            "VEI" => Self::Vei,
            "VIhA" => Self::Viha,
            "VUhU" => Self::Vuhu,
            "XI" => Self::Xi,
            "Y" => Self::Y,
            "ZAhO" => Self::Zaho,
            "ZEhA" => Self::Zeha,
            "ZEI" => Self::Zei,
            "ZI" => Self::Zi,
            "ZO" => Self::Zo,
            "ZOhU" => Self::Zohu,
            "ZOI" => Self::Zoi,
            _ => return None,
        })
    }

    #[requires(true)]
    #[bityzba::ensures(true)]
    pub const fn contains(self, cmavo: Cmavo) -> bool {
        match self {
            Self::A => matches!(cmavo, Cmavo::A | Cmavo::E | Cmavo::Ji | Cmavo::O | Cmavo::U),
            Self::Bahe => matches!(cmavo, Cmavo::Bahe | Cmavo::Zahe),
            Self::Bai => matches!(
                cmavo,
                Cmavo::Bahi
                    | Cmavo::Bai
                    | Cmavo::Baihau
                    | Cmavo::Bau
                    | Cmavo::Behau
                    | Cmavo::Behei
                    | Cmavo::Behi
                    | Cmavo::Buhuhe
                    | Cmavo::Cahi
                    | Cmavo::Cau
                    | Cmavo::Cihe
                    | Cmavo::Ciho
                    | Cmavo::Cihu
                    | Cmavo::Cuhei
                    | Cmavo::Cuhu
                    | Cmavo::Dauha
                    | Cmavo::Dauho
                    | Cmavo::Dauhu
                    | Cmavo::Dehahu
                    | Cmavo::Dehi
                    | Cmavo::Dehiha
                    | Cmavo::Dehihe
                    | Cmavo::Dehihi
                    | Cmavo::Dehiho
                    | Cmavo::Dehihu
                    | Cmavo::Diho
                    | Cmavo::Dohe
                    | Cmavo::Duhi
                    | Cmavo::Duho
                    | Cmavo::Ehuhi
                    | Cmavo::Eihei
                    | Cmavo::Fahe
                    | Cmavo::Fau
                    | Cmavo::Fauhu
                    | Cmavo::Fihe
                    | Cmavo::Gaha
                    | Cmavo::Gahei
                    | Cmavo::Gau
                    | Cmavo::Jahau
                    | Cmavo::Jahe
                    | Cmavo::Jahi
                    | Cmavo::Jahoi
                    | Cmavo::Jahui
                    | Cmavo::Jihe
                    | Cmavo::Jihehe
                    | Cmavo::Jihiha
                    | Cmavo::Jiho
                    | Cmavo::Jihu
                    | Cmavo::Kaha
                    | Cmavo::Kahai
                    | Cmavo::Kahi
                    | Cmavo::Kai
                    | Cmavo::Kihai
                    | Cmavo::Kihi
                    | Cmavo::Kihohe
                    | Cmavo::Kihoi
                    | Cmavo::Kihu
                    | Cmavo::Kihuhe
                    | Cmavo::Kihuhi
                    | Cmavo::Kohau
                    | Cmavo::Koi
                    | Cmavo::Kuhu
                    | Cmavo::Lahai
                    | Cmavo::Lahei
                    | Cmavo::Lahoho
                    | Cmavo::Lahu
                    | Cmavo::Leha
                    | Cmavo::Lihe
                    | Cmavo::Lihehe
                    | Cmavo::Lihei
                    | Cmavo::Mahe
                    | Cmavo::Mahei
                    | Cmavo::Mahi
                    | Cmavo::Mau
                    | Cmavo::Mauhi
                    | Cmavo::Mauhu
                    | Cmavo::Meha
                    | Cmavo::Mehe
                    | Cmavo::Muhai
                    | Cmavo::Muhei
                    | Cmavo::Muhi
                    | Cmavo::Muhoi
                    | Cmavo::Muhu
                    | Cmavo::Nehahi
                    | Cmavo::Nihi
                    | Cmavo::Nihihi
                    | Cmavo::Paha
                    | Cmavo::Pahahi
                    | Cmavo::Pahu
                    | Cmavo::Pehahi
                    | Cmavo::Piho
                    | Cmavo::Pohi
                    | Cmavo::Puha
                    | Cmavo::Puhe
                    | Cmavo::Puhehi
                    | Cmavo::Puhiha
                    | Cmavo::Puhihi
                    | Cmavo::Puhohi
                    | Cmavo::Raha
                    | Cmavo::Rahi
                    | Cmavo::Rai
                    | Cmavo::Raihe
                    | Cmavo::Riha
                    | Cmavo::Rihi
                    | Cmavo::Rihiha
                    | Cmavo::Rihihe
                    | Cmavo::Rihihi
                    | Cmavo::Rihiho
                    | Cmavo::Rihihu
                    | Cmavo::Sau
                    | Cmavo::Sihu
                    | Cmavo::Tahi
                    | Cmavo::Tahiha
                    | Cmavo::Tahihe
                    | Cmavo::Tahihi
                    | Cmavo::Tahiho
                    | Cmavo::Tahihu
                    | Cmavo::Tahuhi
                    | Cmavo::Tai
                    | Cmavo::Tehai
                    | Cmavo::Tihi
                    | Cmavo::Tihiha
                    | Cmavo::Tihu
                    | Cmavo::Tihuha
                    | Cmavo::Tihuhi
                    | Cmavo::Tihuhu
                    | Cmavo::Tuhi
                    | Cmavo::Tuhiha
                    | Cmavo::Tuhihe
                    | Cmavo::Tuhihi
                    | Cmavo::Tuhiho
                    | Cmavo::Tuhihu
                    | Cmavo::Vaho
                    | Cmavo::Vahohi
                    | Cmavo::Vahu
                    | Cmavo::Xuhai
                    | Cmavo::Zau
                    | Cmavo::Zauha
                    | Cmavo::Zauhe
                    | Cmavo::Zauhi
                    | Cmavo::Zauho
                    | Cmavo::Zauhu
                    | Cmavo::Zuhai
                    | Cmavo::Zuhe
            ),
            Self::Be => matches!(cmavo, Cmavo::Be),
            Self::Beho => matches!(cmavo, Cmavo::Beho),
            Self::Bei => matches!(cmavo, Cmavo::Bei),
            Self::Bihi => matches!(cmavo, Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi),
            Self::Bu => matches!(cmavo, Cmavo::Bu),
            Self::By => matches!(
                cmavo,
                Cmavo::A
                    | Cmavo::Ahy
                    | Cmavo::By
                    | Cmavo::Cauhe
                    | Cmavo::Cauhi
                    | Cmavo::Cy
                    | Cmavo::Daiha
                    | Cmavo::Daihe
                    | Cmavo::Daihi
                    | Cmavo::Daiho
                    | Cmavo::Daihu
                    | Cmavo::Daihy
                    | Cmavo::Dauhe
                    | Cmavo::Dauhi
                    | Cmavo::Dy
                    | Cmavo::E
                    | Cmavo::Ehy
                    | Cmavo::Fauha
                    | Cmavo::Fauhe
                    | Cmavo::Fauhi
                    | Cmavo::Fauho
                    | Cmavo::Fauhu
                    | Cmavo::Fy
                    | Cmavo::Gahe
                    | Cmavo::Gaiha
                    | Cmavo::Gaihe
                    | Cmavo::Gaihi
                    | Cmavo::Gaiho
                    | Cmavo::Gaihu
                    | Cmavo::Geho
                    | Cmavo::Gy
                    | Cmavo::I
                    | Cmavo::Ihy
                    | Cmavo::Iy
                    | Cmavo::Jauha
                    | Cmavo::Jauhe
                    | Cmavo::Jauhi
                    | Cmavo::Jauho
                    | Cmavo::Jauhu
                    | Cmavo::Jeho
                    | Cmavo::Joho
                    | Cmavo::Joiho
                    | Cmavo::Joihu
                    | Cmavo::Jy
                    | Cmavo::Kauha
                    | Cmavo::Kauhe
                    | Cmavo::Kauhi
                    | Cmavo::Kauho
                    | Cmavo::Kauhu
                    | Cmavo::Ky
                    | Cmavo::Loha
                    | Cmavo::Ly
                    | Cmavo::My
                    | Cmavo::Naha
                    | Cmavo::Ny
                    | Cmavo::O
                    | Cmavo::Ohy
                    | Cmavo::Py
                    | Cmavo::Ruho
                    | Cmavo::Ry
                    | Cmavo::Sy
                    | Cmavo::Toha
                    | Cmavo::Ty
                    | Cmavo::U
                    | Cmavo::Uhy
                    | Cmavo::Uy
                    | Cmavo::Vy
                    | Cmavo::Xy
                    | Cmavo::Yhy
                    | Cmavo::Zy
            ),
            Self::Caha => matches!(
                cmavo,
                Cmavo::Bihai | Cmavo::Caha | Cmavo::Kahe | Cmavo::Nuho | Cmavo::Puhi
            ),
            Self::Cai => matches!(
                cmavo,
                Cmavo::Cai | Cmavo::Cuhi | Cmavo::Pei | Cmavo::Ruhe | Cmavo::Sai
            ),
            Self::Cehe => matches!(cmavo, Cmavo::Cehe),
            Self::Co => matches!(cmavo, Cmavo::Co),
            Self::Coi => matches!(
                cmavo,
                Cmavo::Ahoi
                    | Cmavo::Behe
                    | Cmavo::Coho
                    | Cmavo::Cohoi
                    | Cmavo::Coi
                    | Cmavo::Dihai
                    | Cmavo::Feho
                    | Cmavo::Feihe
                    | Cmavo::Fihi
                    | Cmavo::Gauhi
                    | Cmavo::Jehe
                    | Cmavo::Jeihe
                    | Cmavo::Juhi
                    | Cmavo::Keho
                    | Cmavo::Kihai
                    | Cmavo::Kihe
                    | Cmavo::Mihe
                    | Cmavo::Mihei
                    | Cmavo::Muho
                    | Cmavo::Nuhe
                    | Cmavo::Ohai
                    | Cmavo::Pehei
                    | Cmavo::Pehu
                    | Cmavo::Peihe
                    | Cmavo::Rehei
                    | Cmavo::Rehi
                    | Cmavo::Sahei
                    | Cmavo::Taha
                    | Cmavo::Viho
                    | Cmavo::Xuhei
            ),
            Self::Cu => matches!(cmavo, Cmavo::Cu),
            Self::Cuhe => matches!(
                cmavo,
                Cmavo::Bahau | Cmavo::Cuhe | Cmavo::Nau | Cmavo::Puhau
            ),
            Self::Daho => matches!(cmavo, Cmavo::Daiho | Cmavo::Dohai),
            Self::Doi => matches!(cmavo, Cmavo::Dahei | Cmavo::Dahoi),
            Self::Fa => matches!(
                cmavo,
                Cmavo::Fa
                    | Cmavo::Fai
                    | Cmavo::Fe
                    | Cmavo::Fi
                    | Cmavo::Fiha
                    | Cmavo::Fo
                    | Cmavo::Fu
            ),
            Self::Faha => matches!(
                cmavo,
                Cmavo::Beha
                    | Cmavo::Buhu
                    | Cmavo::Cahu
                    | Cmavo::Duha
                    | Cmavo::Duhoi
                    | Cmavo::Faha
                    | Cmavo::Gahu
                    | Cmavo::Neha
                    | Cmavo::Nehi
                    | Cmavo::Nehu
                    | Cmavo::Niha
                    | Cmavo::Paho
                    | Cmavo::Reho
                    | Cmavo::Rihu
                    | Cmavo::Ruhu
                    | Cmavo::Tehe
                    | Cmavo::Tiha
                    | Cmavo::Toho
                    | Cmavo::Vuha
                    | Cmavo::Xeihe
                    | Cmavo::Zeho
                    | Cmavo::Zoha
                    | Cmavo::Zohi
                    | Cmavo::Zuha
                    | Cmavo::Zuhau
            ),
            Self::Faho => matches!(cmavo, Cmavo::Faho),
            Self::Fuha => matches!(cmavo, Cmavo::Fuha),
            Self::Ga => matches!(
                cmavo,
                Cmavo::Ga | Cmavo::Ge | Cmavo::Gehi | Cmavo::Go | Cmavo::Gu
            ),
            Self::Gaho => matches!(cmavo, Cmavo::Gaho | Cmavo::Kehi),
            Self::Gi => matches!(cmavo, Cmavo::Gi),
            Self::Giha => matches!(
                cmavo,
                Cmavo::Giha | Cmavo::Gihe | Cmavo::Gihi | Cmavo::Giho | Cmavo::Gihu
            ),
            Self::Gihi => matches!(cmavo, Cmavo::Gihi),
            Self::Goha => matches!(
                cmavo,
                Cmavo::Buha
                    | Cmavo::Buhe
                    | Cmavo::Buhi
                    | Cmavo::Ceihi
                    | Cmavo::Cohe
                    | Cmavo::Du
                    | Cmavo::Gaiho
                    | Cmavo::Goha
                    | Cmavo::Gohe
                    | Cmavo::Gohi
                    | Cmavo::Goho
                    | Cmavo::Gohu
                    | Cmavo::Mo
                    | Cmavo::Nei
                    | Cmavo::Noha
                    | Cmavo::Xehu
            ),
            Self::Goi => matches!(
                cmavo,
                Cmavo::Goi
                    | Cmavo::Ne
                    | Cmavo::Nohu
                    | Cmavo::Pe
                    | Cmavo::Po
                    | Cmavo::Pohe
                    | Cmavo::Pohu
                    | Cmavo::Voihe
            ),
            Self::Guha => matches!(
                cmavo,
                Cmavo::Guha | Cmavo::Guhe | Cmavo::Guhi | Cmavo::Guho | Cmavo::Guhu
            ),
            Self::I => matches!(cmavo, Cmavo::I),
            Self::Ja => matches!(
                cmavo,
                Cmavo::Ja | Cmavo::Je | Cmavo::Jehi | Cmavo::Jo | Cmavo::Ju
            ),
            Self::Jai => matches!(cmavo, Cmavo::Jahei | Cmavo::Johai),
            Self::Jehi => matches!(
                cmavo,
                Cmavo::Ja | Cmavo::Je | Cmavo::Jehi | Cmavo::Jo | Cmavo::Ju
            ),
            Self::Johi => matches!(cmavo, Cmavo::Johi),
            Self::Joi => matches!(
                cmavo,
                Cmavo::Ce
                    | Cmavo::Ceho
                    | Cmavo::Fahu
                    | Cmavo::Jauhu
                    | Cmavo::Jehau
                    | Cmavo::Jeihi
                    | Cmavo::Jeiho
                    | Cmavo::Johau
                    | Cmavo::Johe
                    | Cmavo::Johiha
                    | Cmavo::Johu
                    | Cmavo::Johuhu
                    | Cmavo::Joi
                    | Cmavo::Joihe
                    | Cmavo::Juhe
                    | Cmavo::Kuha
                    | Cmavo::Pihu
            ),
            Self::Koha => matches!(
                cmavo,
                Cmavo::Cehu
                    | Cmavo::Da
                    | Cmavo::Dahe
                    | Cmavo::Dahei
                    | Cmavo::Dahu
                    | Cmavo::De
                    | Cmavo::Dehe
                    | Cmavo::Dehu
                    | Cmavo::Dei
                    | Cmavo::Deiha
                    | Cmavo::Di
                    | Cmavo::Dihe
                    | Cmavo::Dihei
                    | Cmavo::Dihu
                    | Cmavo::Do
                    | Cmavo::Dohi
                    | Cmavo::Doho
                    | Cmavo::Foha
                    | Cmavo::Fohai
                    | Cmavo::Fohe
                    | Cmavo::Fohi
                    | Cmavo::Foho
                    | Cmavo::Fohu
                    | Cmavo::Keha
                    | Cmavo::Kihaha
                    | Cmavo::Kiheha
                    | Cmavo::Kihiha
                    | Cmavo::Kihoha
                    | Cmavo::Kihuha
                    | Cmavo::Ko
                    | Cmavo::Koha
                    | Cmavo::Kohe
                    | Cmavo::Kohi
                    | Cmavo::Koho
                    | Cmavo::Kohu
                    | Cmavo::Ma
                    | Cmavo::Maha
                    | Cmavo::Mahau
                    | Cmavo::Mahei
                    | Cmavo::Mahoi
                    | Cmavo::Mi
                    | Cmavo::Miha
                    | Cmavo::Mihai
                    | Cmavo::Mihau
                    | Cmavo::Miho
                    | Cmavo::Moho
                    | Cmavo::Mohu
                    | Cmavo::Nauho
                    | Cmavo::Nauhu
                    | Cmavo::Ra
                    | Cmavo::Rahai
                    | Cmavo::Rauhi
                    | Cmavo::Ri
                    | Cmavo::Rohei
                    | Cmavo::Ru
                    | Cmavo::Sehe
                    | Cmavo::Sohai
                    | Cmavo::Ta
                    | Cmavo::Ti
                    | Cmavo::Tihau
                    | Cmavo::Tohohe
                    | Cmavo::Tu
                    | Cmavo::Tuhau
                    | Cmavo::Voha
                    | Cmavo::Vohe
                    | Cmavo::Vohi
                    | Cmavo::Voho
                    | Cmavo::Vohu
                    | Cmavo::Xai
                    | Cmavo::Ziho
                    | Cmavo::Zohe
                    | Cmavo::Zohei
                    | Cmavo::Zuhai
                    | Cmavo::Zuhi
            ),
            Self::Ku => matches!(cmavo, Cmavo::Ku),
            Self::La => matches!(cmavo, Cmavo::La | Cmavo::Lahi | Cmavo::Lai),
            Self::Lahe => matches!(
                cmavo,
                Cmavo::Lahe
                    | Cmavo::Loihe
                    | Cmavo::Loihi
                    | Cmavo::Luha
                    | Cmavo::Luhe
                    | Cmavo::Luhi
                    | Cmavo::Luho
                    | Cmavo::Mehohe
                    | Cmavo::Pihei
                    | Cmavo::Pohoi
                    | Cmavo::Poihei
                    | Cmavo::Tehoi
                    | Cmavo::Tuha
                    | Cmavo::Voihe
                    | Cmavo::Vuhi
                    | Cmavo::Zohei
            ),
            Self::Lau => matches!(cmavo, Cmavo::Ceha | Cmavo::Lau | Cmavo::Tau | Cmavo::Zai),
            Self::Le => matches!(
                cmavo,
                Cmavo::Lahei
                    | Cmavo::Le
                    | Cmavo::Lehe
                    | Cmavo::Lehei
                    | Cmavo::Lehi
                    | Cmavo::Lei
                    | Cmavo::Leihe
                    | Cmavo::Leihi
                    | Cmavo::Lo
                    | Cmavo::Lohe
                    | Cmavo::Lohei
                    | Cmavo::Lohi
                    | Cmavo::Loi
                    | Cmavo::Loihe
                    | Cmavo::Loihi
                    | Cmavo::Mehei
                    | Cmavo::Mohoi
                    | Cmavo::Moihoi
                    | Cmavo::Rihoi
                    | Cmavo::Zohau
            ),
            Self::Lehu => matches!(cmavo, Cmavo::Lehu),
            Self::Li => matches!(
                cmavo,
                Cmavo::Bohai | Cmavo::Li | Cmavo::Lihai | Cmavo::Lihei | Cmavo::Maiho | Cmavo::Meho
            ),
            Self::Lihau => matches!(cmavo, Cmavo::Lihau),
            Self::Lihu => matches!(cmavo, Cmavo::Lihu),
            Self::Loho => matches!(cmavo, Cmavo::Loho),
            Self::Lohoi => matches!(
                cmavo,
                Cmavo::Lohoi | Cmavo::Mauha | Cmavo::Xauha | Cmavo::Xuhu
            ),
            Self::Lohu => matches!(cmavo, Cmavo::Lohu),
            Self::Lu => matches!(cmavo, Cmavo::Lahau | Cmavo::Lu | Cmavo::Tuhai),
            Self::Luhei => matches!(cmavo, Cmavo::Luhei),
            Self::Mai => matches!(cmavo, Cmavo::Mai | Cmavo::Moho),
            Self::Me => matches!(cmavo, Cmavo::Mehau | Cmavo::Xohi),
            Self::Mohe => matches!(cmavo, Cmavo::Boihau),
            Self::Mohi => matches!(cmavo, Cmavo::Mohi),
            Self::Moi => matches!(
                cmavo,
                Cmavo::Ceiha
                    | Cmavo::Cuho
                    | Cmavo::Mei
                    | Cmavo::Moi
                    | Cmavo::Moiho
                    | Cmavo::Sihe
                    | Cmavo::Vahe
            ),
            Self::Na => matches!(cmavo, Cmavo::Jaha | Cmavo::Na),
            Self::Nahe => matches!(
                cmavo,
                Cmavo::Dehai
                    | Cmavo::Jeha
                    | Cmavo::Nahe
                    | Cmavo::Nahei
                    | Cmavo::Nohe
                    | Cmavo::Nohei
                    | Cmavo::Tohe
            ),
            Self::Nai => matches!(cmavo, Cmavo::Jahai | Cmavo::Nai),
            Self::Niho => matches!(cmavo, Cmavo::Niho | Cmavo::Nohi),
            Self::Noi => matches!(
                cmavo,
                Cmavo::Nohoi | Cmavo::Noi | Cmavo::Pohoi | Cmavo::Poi | Cmavo::Voi | Cmavo::Voihi
            ),
            Self::Noiha => matches!(
                cmavo,
                Cmavo::Noiha | Cmavo::Noihoha | Cmavo::Poiha | Cmavo::Poihoha | Cmavo::Soiha
            ),
            Self::Nu => matches!(
                cmavo,
                Cmavo::Duhu
                    | Cmavo::Jahoi
                    | Cmavo::Jei
                    | Cmavo::Ka
                    | Cmavo::Kahai
                    | Cmavo::Kaihai
                    | Cmavo::Kaihu
                    | Cmavo::Kihi
                    | Cmavo::Lihi
                    | Cmavo::Muhe
                    | Cmavo::Ni
                    | Cmavo::Nu
                    | Cmavo::Paihe
                    | Cmavo::Poihi
                    | Cmavo::Puhu
                    | Cmavo::Siho
                    | Cmavo::Suhai
                    | Cmavo::Suhu
                    | Cmavo::Xehei
                    | Cmavo::Zahai
                    | Cmavo::Zahi
                    | Cmavo::Zuho
            ),
            Self::Pa => matches!(
                cmavo,
                Cmavo::Bi
                    | Cmavo::Cehi
                    | Cmavo::Ci
                    | Cmavo::Cihi
                    | Cmavo::Daha
                    | Cmavo::Dau
                    | Cmavo::Digit0
                    | Cmavo::Digit1
                    | Cmavo::Digit2
                    | Cmavo::Digit3
                    | Cmavo::Digit4
                    | Cmavo::Digit5
                    | Cmavo::Digit6
                    | Cmavo::Digit7
                    | Cmavo::Digit8
                    | Cmavo::Digit9
                    | Cmavo::Duhe
                    | Cmavo::Duhei
                    | Cmavo::Faihu
                    | Cmavo::Fei
                    | Cmavo::Fihu
                    | Cmavo::Gai
                    | Cmavo::Jau
                    | Cmavo::Jihi
                    | Cmavo::Kaho
                    | Cmavo::Kiho
                    | Cmavo::Mahu
                    | Cmavo::Mehei
                    | Cmavo::Mehi
                    | Cmavo::Moha
                    | Cmavo::Mu
                    | Cmavo::Nihu
                    | Cmavo::No
                    | Cmavo::Noho
                    | Cmavo::Pa
                    | Cmavo::Pai
                    | Cmavo::Pi
                    | Cmavo::Pihe
                    | Cmavo::Rahe
                    | Cmavo::Rau
                    | Cmavo::Re
                    | Cmavo::Rei
                    | Cmavo::Ro
                    | Cmavo::Rohoi
                    | Cmavo::So
                    | Cmavo::Soha
                    | Cmavo::Sohai
                    | Cmavo::Sohe
                    | Cmavo::Sohei
                    | Cmavo::Sohi
                    | Cmavo::Soho
                    | Cmavo::Sohoi
                    | Cmavo::Sohu
                    | Cmavo::Suhai
                    | Cmavo::Suhe
                    | Cmavo::Suho
                    | Cmavo::Suhoi
                    | Cmavo::Teho
                    | Cmavo::Tuho
                    | Cmavo::Vai
                    | Cmavo::Vo
                    | Cmavo::Xa
                    | Cmavo::Xaihe
                    | Cmavo::Xauhe
                    | Cmavo::Xehe
                    | Cmavo::Xo
                    | Cmavo::Xohai
                    | Cmavo::Xohe
                    | Cmavo::Xohu
                    | Cmavo::Xoihi
                    | Cmavo::Zahai
                    | Cmavo::Zahu
                    | Cmavo::Ze
            ),
            Self::Pehe => matches!(cmavo, Cmavo::Pehe),
            Self::Pu => matches!(cmavo, Cmavo::Ba | Cmavo::Ca | Cmavo::Pu),
            Self::Roi => matches!(
                cmavo,
                Cmavo::Bahoi
                    | Cmavo::Dehei
                    | Cmavo::Muhei
                    | Cmavo::Rehu
                    | Cmavo::Roi
                    | Cmavo::Vahei
                    | Cmavo::Xuhau
            ),
            Self::Sa => matches!(cmavo, Cmavo::Sa),
            Self::Se => matches!(
                cmavo,
                Cmavo::Dehai
                    | Cmavo::Nahoi
                    | Cmavo::Se
                    | Cmavo::Suhei
                    | Cmavo::Te
                    | Cmavo::Tohai
                    | Cmavo::Ve
                    | Cmavo::Vohai
                    | Cmavo::Xe
                    | Cmavo::Xohai
            ),
            Self::Sehu => matches!(cmavo, Cmavo::Xehau),
            Self::Sei => matches!(
                cmavo,
                Cmavo::Saihe
                    | Cmavo::Sei
                    | Cmavo::Seihe
                    | Cmavo::Soihe
                    | Cmavo::Suhoi
                    | Cmavo::Tiho
            ),
            Self::Si => matches!(cmavo, Cmavo::Si),
            Self::Soi => matches!(cmavo, Cmavo::Soi | Cmavo::Xoi),
            Self::Su => matches!(cmavo, Cmavo::Su),
            Self::Tahe => matches!(cmavo, Cmavo::Dihi | Cmavo::Naho | Cmavo::Ruhi | Cmavo::Tahe),
            Self::To => matches!(cmavo, Cmavo::Mauhe | Cmavo::Noihi | Cmavo::To | Cmavo::Tohi),
            Self::Toi => matches!(cmavo, Cmavo::Gehuhi | Cmavo::Mauho | Cmavo::Toi),
            Self::Tuhe => matches!(cmavo, Cmavo::Tuhe),
            Self::Ui => matches!(
                cmavo,
                Cmavo::Aha
                    | Cmavo::Ahai
                    | Cmavo::Ahe
                    | Cmavo::Ahi
                    | Cmavo::Aho
                    | Cmavo::Ahu
                    | Cmavo::Ai
                    | Cmavo::Aihi
                    | Cmavo::Au
                    | Cmavo::Auhau
                    | Cmavo::Baha
                    | Cmavo::Bahei
                    | Cmavo::Bahu
                    | Cmavo::Behu
                    | Cmavo::Bihu
                    | Cmavo::Buhei
                    | Cmavo::Buho
                    | Cmavo::Cahe
                    | Cmavo::Cuhei
                    | Cmavo::Dahi
                    | Cmavo::Daho
                    | Cmavo::Dai
                    | Cmavo::Doha
                    | Cmavo::Eha
                    | Cmavo::Ehe
                    | Cmavo::Ehei
                    | Cmavo::Ehi
                    | Cmavo::Eho
                    | Cmavo::Ehu
                    | Cmavo::Ei
                    | Cmavo::Eihai
                    | Cmavo::Fahai
                    | Cmavo::Fuhau
                    | Cmavo::Fuhe
                    | Cmavo::Fuhi
                    | Cmavo::Fuho
                    | Cmavo::Gahi
                    | Cmavo::Gahihi
                    | Cmavo::Gahuhi
                    | Cmavo::Gehai
                    | Cmavo::Gehe
                    | Cmavo::Ia
                    | Cmavo::Iahau
                    | Cmavo::Ie
                    | Cmavo::Iha
                    | Cmavo::Ihau
                    | Cmavo::Ihe
                    | Cmavo::Ihei
                    | Cmavo::Ihi
                    | Cmavo::Ihihi
                    | Cmavo::Iho
                    | Cmavo::Ihu
                    | Cmavo::Ii
                    | Cmavo::Io
                    | Cmavo::Iu
                    | Cmavo::Jaho
                    | Cmavo::Jahohe
                    | Cmavo::Jahoho
                    | Cmavo::Jehu
                    | Cmavo::Jiha
                    | Cmavo::Jihai
                    | Cmavo::Jihei
                    | Cmavo::Jihohe
                    | Cmavo::Jihoho
                    | Cmavo::Joha
                    | Cmavo::Juha
                    | Cmavo::Juho
                    | Cmavo::Juhoi
                    | Cmavo::Kahu
                    | Cmavo::Kau
                    | Cmavo::Kehihai
                    | Cmavo::Kehu
                    | Cmavo::Kiha
                    | Cmavo::Kihai
                    | Cmavo::Kohoi
                    | Cmavo::Kuhi
                    | Cmavo::Laha
                    | Cmavo::Lahei
                    | Cmavo::Lahoi
                    | Cmavo::Leho
                    | Cmavo::Lehohe
                    | Cmavo::Liha
                    | Cmavo::Liho
                    | Cmavo::Lihoi
                    | Cmavo::Mahai
                    | Cmavo::Mihu
                    | Cmavo::Muha
                    | Cmavo::Muhei
                    | Cmavo::Nahi
                    | Cmavo::Nihei
                    | Cmavo::Nohoi
                    | Cmavo::Oha
                    | Cmavo::Ohe
                    | Cmavo::Ohi
                    | Cmavo::Oho
                    | Cmavo::Ohu
                    | Cmavo::Oi
                    | Cmavo::Oiha
                    | Cmavo::Oihoi
                    | Cmavo::Pahe
                    | Cmavo::Pau
                    | Cmavo::Peha
                    | Cmavo::Pehi
                    | Cmavo::Pohai
                    | Cmavo::Poho
                    | Cmavo::Rahu
                    | Cmavo::Rehe
                    | Cmavo::Rihe
                    | Cmavo::Roha
                    | Cmavo::Rohe
                    | Cmavo::Rohi
                    | Cmavo::Roho
                    | Cmavo::Rohu
                    | Cmavo::Ruha
                    | Cmavo::Saha
                    | Cmavo::Sahe
                    | Cmavo::Sahu
                    | Cmavo::Saihi
                    | Cmavo::Seha
                    | Cmavo::Sehi
                    | Cmavo::Seho
                    | Cmavo::Seiha
                    | Cmavo::Seihi
                    | Cmavo::Siha
                    | Cmavo::Sihau
                    | Cmavo::Sohahu
                    | Cmavo::Sohei
                    | Cmavo::Suha
                    | Cmavo::Suhei
                    | Cmavo::Taho
                    | Cmavo::Tahu
                    | Cmavo::Tihe
                    | Cmavo::Tohu
                    | Cmavo::Ua
                    | Cmavo::Ue
                    | Cmavo::Uehi
                    | Cmavo::Uha
                    | Cmavo::Uhe
                    | Cmavo::Uhi
                    | Cmavo::Uho
                    | Cmavo::Uhohe
                    | Cmavo::Uhohi
                    | Cmavo::Uhoho
                    | Cmavo::Uhohu
                    | Cmavo::Uhoi
                    | Cmavo::Uhu
                    | Cmavo::Ui
                    | Cmavo::Uihai
                    | Cmavo::Uo
                    | Cmavo::Uu
                    | Cmavo::Vahi
                    | Cmavo::Vaihe
                    | Cmavo::Vuhe
                    | Cmavo::Xauha
                    | Cmavo::Xauhe
                    | Cmavo::Xauhi
                    | Cmavo::Xauho
                    | Cmavo::Xauhu
                    | Cmavo::Xehiha
                    | Cmavo::Xehihe
                    | Cmavo::Xehihi
                    | Cmavo::Xehiho
                    | Cmavo::Xehihu
                    | Cmavo::Xoho
                    | Cmavo::Xu
                    | Cmavo::Zaha
                    | Cmavo::Zahei
                    | Cmavo::Zahoha
                    | Cmavo::Zoho
                    | Cmavo::Zohoi
                    | Cmavo::Zuhu
            ),
            Self::Ui3a => matches!(
                cmavo,
                Cmavo::Ahai
                    | Cmavo::Auhau
                    | Cmavo::Bahei
                    | Cmavo::Buhei
                    | Cmavo::Cuhei
                    | Cmavo::Eihai
                    | Cmavo::Fahai
                    | Cmavo::Gahihi
                    | Cmavo::Gahuhi
                    | Cmavo::Gehai
                    | Cmavo::Iahau
                    | Cmavo::Ihau
                    | Cmavo::Ihei
                    | Cmavo::Ihihi
                    | Cmavo::Jahohe
                    | Cmavo::Jahoho
                    | Cmavo::Jihai
                    | Cmavo::Jihei
                    | Cmavo::Jihohe
                    | Cmavo::Jihoho
                    | Cmavo::Kehihai
                    | Cmavo::Kihai
                    | Cmavo::Lahei
                    | Cmavo::Lahoi
                    | Cmavo::Lehohe
                    | Cmavo::Mahai
                    | Cmavo::Muhei
                    | Cmavo::Nihei
                    | Cmavo::Nohoi
                    | Cmavo::Oihoi
                    | Cmavo::Pohai
                    | Cmavo::Saihi
                    | Cmavo::Seiha
                    | Cmavo::Seihi
                    | Cmavo::Sohahu
                    | Cmavo::Sohei
                    | Cmavo::Suhei
                    | Cmavo::Uhohe
                    | Cmavo::Uhohi
                    | Cmavo::Uhoho
                    | Cmavo::Uhohu
                    | Cmavo::Uhoi
                    | Cmavo::Uihai
                    | Cmavo::Vaihe
                    | Cmavo::Xauha
                    | Cmavo::Xauhe
                    | Cmavo::Xauhi
                    | Cmavo::Xauho
                    | Cmavo::Xauhu
                    | Cmavo::Xehiha
                    | Cmavo::Xehihe
                    | Cmavo::Xehihi
                    | Cmavo::Xehiho
                    | Cmavo::Xehihu
                    | Cmavo::Zahei
                    | Cmavo::Zahoha
                    | Cmavo::Zohoi
            ),
            Self::Va => matches!(cmavo, Cmavo::Va | Cmavo::Vi | Cmavo::Vu),
            Self::Vau => matches!(cmavo, Cmavo::Vau),
            Self::Veha => matches!(cmavo, Cmavo::Veha | Cmavo::Vehe | Cmavo::Vehi | Cmavo::Vehu),
            Self::Veho => matches!(cmavo, Cmavo::Veho),
            Self::Vei => matches!(cmavo, Cmavo::Vei),
            Self::Viha => matches!(cmavo, Cmavo::Viha | Cmavo::Vihe | Cmavo::Vihi | Cmavo::Vihu),
            Self::Vuhu => matches!(
                cmavo,
                Cmavo::Cuha
                    | Cmavo::Deho
                    | Cmavo::Dehoha
                    | Cmavo::Fahi
                    | Cmavo::Feha
                    | Cmavo::Fehaha
                    | Cmavo::Fehahe
                    | Cmavo::Fehahi
                    | Cmavo::Fehaho
                    | Cmavo::Fehi
                    | Cmavo::Fuhu
                    | Cmavo::Geha
                    | Cmavo::Gei
                    | Cmavo::Geiha
                    | Cmavo::Joihi
                    | Cmavo::Juhu
                    | Cmavo::Neho
                    | Cmavo::Pahi
                    | Cmavo::Piha
                    | Cmavo::Pihai
                    | Cmavo::Pihi
                    | Cmavo::Reha
                    | Cmavo::Riho
                    | Cmavo::Sahi
                    | Cmavo::Sahiha
                    | Cmavo::Saho
                    | Cmavo::Sihi
                    | Cmavo::Suhi
                    | Cmavo::Teha
                    | Cmavo::Vaha
                    | Cmavo::Vuhu
            ),
            Self::Xi => matches!(
                cmavo,
                Cmavo::Fauhe | Cmavo::Tehai | Cmavo::Xi | Cmavo::Xihe | Cmavo::Xihi
            ),
            Self::Y => matches!(cmavo, Cmavo::Ieho | Cmavo::Y),
            Self::Zaho => matches!(
                cmavo,
                Cmavo::Baho
                    | Cmavo::Caho
                    | Cmavo::Coha
                    | Cmavo::Cohaha
                    | Cmavo::Cohauha
                    | Cmavo::Cohi
                    | Cmavo::Cohu
                    | Cmavo::Cohuha
                    | Cmavo::Deha
                    | Cmavo::Diha
                    | Cmavo::Mohu
                    | Cmavo::Puho
                    | Cmavo::Sauha
                    | Cmavo::Xaho
                    | Cmavo::Xohu
                    | Cmavo::Zaho
            ),
            Self::Zeha => matches!(cmavo, Cmavo::Zeha | Cmavo::Zehe | Cmavo::Zehi | Cmavo::Zehu),
            Self::Zei => matches!(cmavo, Cmavo::Zei),
            Self::Zi => matches!(cmavo, Cmavo::Za | Cmavo::Zi | Cmavo::Zu),
            Self::Zo => matches!(cmavo, Cmavo::Mahoi | Cmavo::Zo),
            Self::Zohu => matches!(
                cmavo,
                Cmavo::Cehai | Cmavo::Gehai | Cmavo::Kehau | Cmavo::Zohu
            ),
            Self::Zoi => matches!(cmavo, Cmavo::Laho | Cmavo::Muhoi | Cmavo::Zoi),
        }
    }
}

impl fmt::Display for Selmaho {
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
