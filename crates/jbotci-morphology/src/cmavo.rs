use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

macro_rules! cmavo_table {
    ($callback:ident $(, $arg:expr)*) => {
        $callback! {
            ($($arg),*)
            [
        A => { text: "a", selmaho: [A, By] },
        Aha => { text: "a'a", selmaho: [Ui] },
        Ahai => { text: "a'ai", selmaho: [Ui, Ui3a] },
        Ahe => { text: "a'e", selmaho: [Ui] },
        Ahi => { text: "a'i", selmaho: [Ui] },
        Aho => { text: "a'o", selmaho: [Ui] },
        Ahoi => { text: "a'oi", selmaho: [Coi] },
        Ahu => { text: "a'u", selmaho: [Ui] },
        Ahy => { text: "a'y", selmaho: [By] },
        Ai => { text: "ai", selmaho: [Ui] },
        Aihi => { text: "ai'i", selmaho: [Ui] },
        Au => { text: "au", selmaho: [Ui] },
        Auhau => { text: "au'au", selmaho: [Ui, Ui3a] },
        Ba => { text: "ba", selmaho: [Pu] },
        Baha => { text: "ba'a", selmaho: [Ui] },
        Bahau => { text: "ba'au", selmaho: [Cuhe] },
        Bahe => { text: "ba'e", selmaho: [Bahe] },
        Bahei => { text: "ba'ei", selmaho: [Ui, Ui3a] },
        Bahi => { text: "ba'i", selmaho: [Bai] },
        Baho => { text: "ba'o", selmaho: [Zaho] },
        Bahoi => { text: "ba'oi", selmaho: [Roi] },
        Bahu => { text: "ba'u", selmaho: [Ui] },
        Bai => { text: "bai", selmaho: [Bai] },
        Baihau => { text: "bai'au", selmaho: [Bai] },
        Bau => { text: "bau", selmaho: [Bai] },
        Be => { text: "be", selmaho: [Be] },
        Beha => { text: "be'a", selmaho: [Faha] },
        Behau => { text: "be'au", selmaho: [Bai] },
        Behe => { text: "be'e", selmaho: [Coi] },
        Behei => { text: "be'ei", selmaho: [Bai] },
        Behi => { text: "be'i", selmaho: [Bai] },
        Beho => { text: "be'o", selmaho: [Beho] },
        Behu => { text: "be'u", selmaho: [Ui] },
        Bei => { text: "bei", selmaho: [Bei] },
        Bi => { text: "bi", selmaho: [Pa] },
        Bihai => { text: "bi'ai", selmaho: [Caha] },
        Bihe => { text: "bi'e", selmaho: [] },
        Bihi => { text: "bi'i", selmaho: [Bihi] },
        Biho => { text: "bi'o", selmaho: [Bihi] },
        Bihu => { text: "bi'u", selmaho: [Ui] },
        Bo => { text: "bo", selmaho: [] },
        Bohai => { text: "bo'ai", selmaho: [Li] },
        Bohei => { text: "bo'ei", selmaho: [] },
        Boi => { text: "boi", selmaho: [] },
        Boihau => { text: "boi'au", selmaho: [Mohe] },
        Bu => { text: "bu", selmaho: [Bu] },
        Buha => { text: "bu'a", selmaho: [Goha] },
        Buhe => { text: "bu'e", selmaho: [Goha] },
        Buhei => { text: "bu'ei", selmaho: [Ui, Ui3a] },
        Buhi => { text: "bu'i", selmaho: [Goha] },
        Buho => { text: "bu'o", selmaho: [Ui] },
        Buhu => { text: "bu'u", selmaho: [Faha] },
        Buhuhe => { text: "bu'u'e", selmaho: [Bai] },
        By => { text: "by", selmaho: [By] },
        Ca => { text: "ca", selmaho: [Pu] },
        Caha => { text: "ca'a", selmaho: [Caha] },
        Cahe => { text: "ca'e", selmaho: [Ui] },
        Cahi => { text: "ca'i", selmaho: [Bai] },
        Caho => { text: "ca'o", selmaho: [Zaho] },
        Cahu => { text: "ca'u", selmaho: [Faha] },
        Cai => { text: "cai", selmaho: [Cai] },
        Cau => { text: "cau", selmaho: [Bai] },
        Cauhe => { text: "cau'e", selmaho: [By] },
        Cauhi => { text: "cau'i", selmaho: [By] },
        Ce => { text: "ce", selmaho: [Joi] },
        Ceha => { text: "ce'a", selmaho: [Lau] },
        Cehai => { text: "ce'ai", selmaho: [Zohu] },
        Cehe => { text: "ce'e", selmaho: [Cehe] },
        Cehi => { text: "ce'i", selmaho: [Pa] },
        Ceho => { text: "ce'o", selmaho: [Joi] },
        Cehu => { text: "ce'u", selmaho: [Koha] },
        Cei => { text: "cei", selmaho: [] },
        Ceiha => { text: "cei'a", selmaho: [Moi] },
        Ceihi => { text: "cei'i", selmaho: [Goha] },
        Ci => { text: "ci", selmaho: [Pa] },
        Cihe => { text: "ci'e", selmaho: [Bai] },
        Cihi => { text: "ci'i", selmaho: [Pa] },
        Ciho => { text: "ci'o", selmaho: [Bai] },
        Cihu => { text: "ci'u", selmaho: [Bai] },
        Co => { text: "co", selmaho: [Co] },
        Coha => { text: "co'a", selmaho: [Zaho] },
        Cohaha => { text: "co'a'a", selmaho: [Zaho] },
        Cohauha => { text: "co'au'a", selmaho: [Zaho] },
        Cohe => { text: "co'e", selmaho: [Goha] },
        Cohi => { text: "co'i", selmaho: [Zaho] },
        Coho => { text: "co'o", selmaho: [Coi] },
        Cohoi => { text: "co'oi", selmaho: [Coi] },
        Cohu => { text: "co'u", selmaho: [Zaho] },
        Cohuha => { text: "co'u'a", selmaho: [Zaho] },
        Coi => { text: "coi", selmaho: [Coi] },
        Cu => { text: "cu", selmaho: [Cu] },
        Cuha => { text: "cu'a", selmaho: [Vuhu] },
        Cuhe => { text: "cu'e", selmaho: [Cuhe] },
        Cuhei => { text: "cu'ei", selmaho: [Bai, Ui, Ui3a] },
        Cuhi => { text: "cu'i", selmaho: [Cai] },
        Cuho => { text: "cu'o", selmaho: [Moi] },
        Cuhu => { text: "cu'u", selmaho: [Bai] },
        Cy => { text: "cy", selmaho: [By] },
        Da => { text: "da", selmaho: [Koha] },
        Daha => { text: "da'a", selmaho: [Pa] },
        Dahe => { text: "da'e", selmaho: [Koha] },
        Dahei => { text: "da'ei", selmaho: [Doi, Koha] },
        Dahi => { text: "da'i", selmaho: [Ui] },
        Daho => { text: "da'o", selmaho: [Ui] },
        Dahoi => { text: "da'oi", selmaho: [Doi] },
        Dahu => { text: "da'u", selmaho: [Koha] },
        Dai => { text: "dai", selmaho: [Ui] },
        Daiha => { text: "dai'a", selmaho: [By] },
        Daihe => { text: "dai'e", selmaho: [By] },
        Daihi => { text: "dai'i", selmaho: [By] },
        Daiho => { text: "dai'o", selmaho: [By, Daho] },
        Daihu => { text: "dai'u", selmaho: [By] },
        Daihy => { text: "dai'y", selmaho: [By] },
        Dau => { text: "dau", selmaho: [Pa] },
        Dauha => { text: "dau'a", selmaho: [Bai] },
        Dauhe => { text: "dau'e", selmaho: [By] },
        Dauhi => { text: "dau'i", selmaho: [By] },
        Dauho => { text: "dau'o", selmaho: [Bai] },
        Dauhu => { text: "dau'u", selmaho: [Bai] },
        De => { text: "de", selmaho: [Koha] },
        Deha => { text: "de'a", selmaho: [Zaho] },
        Dehahu => { text: "de'a'u", selmaho: [Bai] },
        Dehai => { text: "de'ai", selmaho: [Nahe, Se] },
        Dehe => { text: "de'e", selmaho: [Koha] },
        Dehei => { text: "de'ei", selmaho: [Roi] },
        Dehi => { text: "de'i", selmaho: [Bai] },
        Dehiha => { text: "de'i'a", selmaho: [Bai] },
        Dehihe => { text: "de'i'e", selmaho: [Bai] },
        Dehihi => { text: "de'i'i", selmaho: [Bai] },
        Dehiho => { text: "de'i'o", selmaho: [Bai] },
        Dehihu => { text: "de'i'u", selmaho: [Bai] },
        Deho => { text: "de'o", selmaho: [Vuhu] },
        Dehoha => { text: "de'o'a", selmaho: [Vuhu] },
        Dehu => { text: "de'u", selmaho: [Koha] },
        Dei => { text: "dei", selmaho: [Koha] },
        Deiha => { text: "dei'a", selmaho: [Koha] },
        Di => { text: "di", selmaho: [Koha] },
        Digit0 => { text: "0", selmaho: [Pa] },
        Digit1 => { text: "1", selmaho: [Pa] },
        Digit2 => { text: "2", selmaho: [Pa] },
        Digit3 => { text: "3", selmaho: [Pa] },
        Digit4 => { text: "4", selmaho: [Pa] },
        Digit5 => { text: "5", selmaho: [Pa] },
        Digit6 => { text: "6", selmaho: [Pa] },
        Digit7 => { text: "7", selmaho: [Pa] },
        Digit8 => { text: "8", selmaho: [Pa] },
        Digit9 => { text: "9", selmaho: [Pa] },
        Diha => { text: "di'a", selmaho: [Zaho] },
        Dihai => { text: "di'ai", selmaho: [Coi] },
        Dihe => { text: "di'e", selmaho: [Koha] },
        Dihei => { text: "di'ei", selmaho: [Koha] },
        Dihi => { text: "di'i", selmaho: [Tahe] },
        Diho => { text: "di'o", selmaho: [Bai] },
        Dihu => { text: "di'u", selmaho: [Koha] },
        Do => { text: "do", selmaho: [Koha] },
        Doha => { text: "do'a", selmaho: [Ui] },
        Dohai => { text: "do'ai", selmaho: [Daho] },
        Dohe => { text: "do'e", selmaho: [Bai] },
        Dohi => { text: "do'i", selmaho: [Koha] },
        Doho => { text: "do'o", selmaho: [Koha] },
        Dohu => { text: "do'u", selmaho: [] },
        Doi => { text: "doi", selmaho: [] },
        Du => { text: "du", selmaho: [Goha] },
        Duha => { text: "du'a", selmaho: [Faha] },
        Duhe => { text: "du'e", selmaho: [Pa] },
        Duhei => { text: "du'ei", selmaho: [Pa] },
        Duhi => { text: "du'i", selmaho: [Bai] },
        Duho => { text: "du'o", selmaho: [Bai] },
        Duhoi => { text: "du'oi", selmaho: [Faha] },
        Duhu => { text: "du'u", selmaho: [Nu] },
        Dy => { text: "dy", selmaho: [By] },
        E => { text: "e", selmaho: [A, By] },
        Eha => { text: "e'a", selmaho: [Ui] },
        Ehe => { text: "e'e", selmaho: [Ui] },
        Ehei => { text: "e'ei", selmaho: [Ui] },
        Ehi => { text: "e'i", selmaho: [Ui] },
        Eho => { text: "e'o", selmaho: [Ui] },
        Ehu => { text: "e'u", selmaho: [Ui] },
        Ehuhi => { text: "e'u'i", selmaho: [Bai] },
        Ehy => { text: "e'y", selmaho: [By] },
        Ei => { text: "ei", selmaho: [Ui] },
        Eihai => { text: "ei'ai", selmaho: [Ui, Ui3a] },
        Eihei => { text: "ei'ei", selmaho: [Bai] },
        Fa => { text: "fa", selmaho: [Fa] },
        Faha => { text: "fa'a", selmaho: [Faha] },
        Fahai => { text: "fa'ai", selmaho: [Ui, Ui3a] },
        Fahe => { text: "fa'e", selmaho: [Bai] },
        Fahi => { text: "fa'i", selmaho: [Vuhu] },
        Faho => { text: "fa'o", selmaho: [Faho] },
        Fahu => { text: "fa'u", selmaho: [Joi] },
        Fai => { text: "fai", selmaho: [Fa] },
        Faihu => { text: "fai'u", selmaho: [Pa] },
        Fau => { text: "fau", selmaho: [Bai] },
        Fauha => { text: "fau'a", selmaho: [By] },
        Fauhe => { text: "fau'e", selmaho: [By, Xi] },
        Fauhi => { text: "fau'i", selmaho: [By] },
        Fauho => { text: "fau'o", selmaho: [By] },
        Fauhu => { text: "fau'u", selmaho: [Bai, By] },
        Fe => { text: "fe", selmaho: [Fa] },
        Feha => { text: "fe'a", selmaho: [Vuhu] },
        Fehaha => { text: "fe'a'a", selmaho: [Vuhu] },
        Fehahe => { text: "fe'a'e", selmaho: [Vuhu] },
        Fehahi => { text: "fe'a'i", selmaho: [Vuhu] },
        Fehaho => { text: "fe'a'o", selmaho: [Vuhu] },
        Fehe => { text: "fe'e", selmaho: [] },
        Fehi => { text: "fe'i", selmaho: [Vuhu] },
        Feho => { text: "fe'o", selmaho: [Coi] },
        Fehu => { text: "fe'u", selmaho: [] },
        Fei => { text: "fei", selmaho: [Pa] },
        Feihe => { text: "fei'e", selmaho: [Coi] },
        Fi => { text: "fi", selmaho: [Fa] },
        Fiha => { text: "fi'a", selmaho: [Fa] },
        Fihau => { text: "fi'au", selmaho: [] },
        Fihe => { text: "fi'e", selmaho: [Bai] },
        Fihi => { text: "fi'i", selmaho: [Coi] },
        Fiho => { text: "fi'o", selmaho: [] },
        Fihoi => { text: "fi'oi", selmaho: [] },
        Fihu => { text: "fi'u", selmaho: [Pa] },
        Fo => { text: "fo", selmaho: [Fa] },
        Foha => { text: "fo'a", selmaho: [Koha] },
        Fohai => { text: "fo'ai", selmaho: [Koha] },
        Fohe => { text: "fo'e", selmaho: [Koha] },
        Fohi => { text: "fo'i", selmaho: [Koha] },
        Foho => { text: "fo'o", selmaho: [Koha] },
        Fohu => { text: "fo'u", selmaho: [Koha] },
        Foi => { text: "foi", selmaho: [] },
        Fu => { text: "fu", selmaho: [Fa] },
        Fuha => { text: "fu'a", selmaho: [Fuha] },
        Fuhau => { text: "fu'au", selmaho: [Ui] },
        Fuhe => { text: "fu'e", selmaho: [Ui] },
        Fuhi => { text: "fu'i", selmaho: [Ui] },
        Fuho => { text: "fu'o", selmaho: [Ui] },
        Fuhu => { text: "fu'u", selmaho: [Vuhu] },
        Fy => { text: "fy", selmaho: [By] },
        Ga => { text: "ga", selmaho: [Ga] },
        Gaha => { text: "ga'a", selmaho: [Bai] },
        Gahe => { text: "ga'e", selmaho: [By] },
        Gahei => { text: "ga'ei", selmaho: [Bai] },
        Gahi => { text: "ga'i", selmaho: [Ui] },
        Gahihi => { text: "ga'i'i", selmaho: [Ui, Ui3a] },
        Gaho => { text: "ga'o", selmaho: [Gaho] },
        Gahu => { text: "ga'u", selmaho: [Faha] },
        Gahuhi => { text: "ga'u'i", selmaho: [Ui, Ui3a] },
        Gai => { text: "gai", selmaho: [Pa] },
        Gaiha => { text: "gai'a", selmaho: [By] },
        Gaihe => { text: "gai'e", selmaho: [By] },
        Gaihi => { text: "gai'i", selmaho: [By] },
        Gaiho => { text: "gai'o", selmaho: [By, Goha] },
        Gaihu => { text: "gai'u", selmaho: [By] },
        Gau => { text: "gau", selmaho: [Bai] },
        Gauhi => { text: "gau'i", selmaho: [Coi] },
        Ge => { text: "ge", selmaho: [Ga] },
        Geha => { text: "ge'a", selmaho: [Vuhu] },
        Gehai => { text: "ge'ai", selmaho: [Ui, Ui3a, Zohu] },
        Gehe => { text: "ge'e", selmaho: [Ui] },
        Gehi => { text: "ge'i", selmaho: [Ga] },
        Geho => { text: "ge'o", selmaho: [By] },
        Gehu => { text: "ge'u", selmaho: [] },
        Gehuhi => { text: "ge'u'i", selmaho: [Toi] },
        Gei => { text: "gei", selmaho: [Vuhu] },
        Geiha => { text: "gei'a", selmaho: [Vuhu] },
        Gi => { text: "gi", selmaho: [Gi] },
        Giha => { text: "gi'a", selmaho: [Giha] },
        Gihe => { text: "gi'e", selmaho: [Giha] },
        Gihi => { text: "gi'i", selmaho: [Giha, Gihi] },
        Giho => { text: "gi'o", selmaho: [Giha] },
        Gihu => { text: "gi'u", selmaho: [Giha] },
        Go => { text: "go", selmaho: [Ga] },
        Goha => { text: "go'a", selmaho: [Goha] },
        Gohe => { text: "go'e", selmaho: [Goha] },
        Gohi => { text: "go'i", selmaho: [Goha] },
        Goho => { text: "go'o", selmaho: [Goha] },
        Gohoi => { text: "go'oi", selmaho: [] },
        Gohu => { text: "go'u", selmaho: [Goha] },
        Goi => { text: "goi", selmaho: [Goi] },
        Gu => { text: "gu", selmaho: [Ga] },
        Guha => { text: "gu'a", selmaho: [Guha] },
        Guhe => { text: "gu'e", selmaho: [Guha] },
        Guhi => { text: "gu'i", selmaho: [Guha] },
        Guho => { text: "gu'o", selmaho: [Guha] },
        Guhu => { text: "gu'u", selmaho: [Guha] },
        Gy => { text: "gy", selmaho: [By] },
        I => { text: "i", selmaho: [By, I] },
        Ia => { text: "ia", selmaho: [Ui] },
        Iahau => { text: "ia'au", selmaho: [Ui, Ui3a] },
        Iahu => { text: "ia'u", selmaho: [] },
        Ie => { text: "ie", selmaho: [Ui] },
        Ieho => { text: "ie'o", selmaho: [Y] },
        Iha => { text: "i'a", selmaho: [Ui] },
        Ihau => { text: "i'au", selmaho: [Ui, Ui3a] },
        Ihe => { text: "i'e", selmaho: [Ui] },
        Ihei => { text: "i'ei", selmaho: [Ui, Ui3a] },
        Ihi => { text: "i'i", selmaho: [Ui] },
        Ihihi => { text: "i'i'i", selmaho: [Ui, Ui3a] },
        Iho => { text: "i'o", selmaho: [Ui] },
        Ihu => { text: "i'u", selmaho: [Ui] },
        Ihy => { text: "i'y", selmaho: [By] },
        Ii => { text: "ii", selmaho: [Ui] },
        Io => { text: "io", selmaho: [Ui] },
        Iu => { text: "iu", selmaho: [Ui] },
        Iy => { text: "iy", selmaho: [By] },
        Ja => { text: "ja", selmaho: [Ja, Jehi] },
        Jaha => { text: "ja'a", selmaho: [Na] },
        Jahai => { text: "ja'ai", selmaho: [Nai] },
        Jahau => { text: "ja'au", selmaho: [Bai] },
        Jahe => { text: "ja'e", selmaho: [Bai] },
        Jahei => { text: "ja'ei", selmaho: [Jai] },
        Jahi => { text: "ja'i", selmaho: [Bai] },
        Jaho => { text: "ja'o", selmaho: [Ui] },
        Jahohe => { text: "ja'o'e", selmaho: [Ui, Ui3a] },
        Jahoho => { text: "ja'o'o", selmaho: [Ui, Ui3a] },
        Jahoi => { text: "ja'oi", selmaho: [Bai, Nu] },
        Jahui => { text: "ja'ui", selmaho: [Bai] },
        Jai => { text: "jai", selmaho: [] },
        Jau => { text: "jau", selmaho: [Pa] },
        Jauha => { text: "jau'a", selmaho: [By] },
        Jauhe => { text: "jau'e", selmaho: [By] },
        Jauhi => { text: "jau'i", selmaho: [By] },
        Jauho => { text: "jau'o", selmaho: [By] },
        Jauhu => { text: "jau'u", selmaho: [By, Joi] },
        Je => { text: "je", selmaho: [Ja, Jehi] },
        Jeha => { text: "je'a", selmaho: [Nahe] },
        Jehau => { text: "je'au", selmaho: [Joi] },
        Jehe => { text: "je'e", selmaho: [Coi] },
        Jehi => { text: "je'i", selmaho: [Ja, Jehi] },
        Jeho => { text: "je'o", selmaho: [By] },
        Jehu => { text: "je'u", selmaho: [Ui] },
        Jei => { text: "jei", selmaho: [Nu] },
        Jeihe => { text: "jei'e", selmaho: [Coi] },
        Jeihi => { text: "jei'i", selmaho: [Joi] },
        Jeiho => { text: "jei'o", selmaho: [Joi] },
        Ji => { text: "ji", selmaho: [A] },
        Jiha => { text: "ji'a", selmaho: [Ui] },
        Jihai => { text: "ji'ai", selmaho: [Ui, Ui3a] },
        Jihe => { text: "ji'e", selmaho: [Bai] },
        Jihehe => { text: "ji'e'e", selmaho: [Bai] },
        Jihei => { text: "ji'ei", selmaho: [Ui, Ui3a] },
        Jihi => { text: "ji'i", selmaho: [Pa] },
        Jihiha => { text: "ji'i'a", selmaho: [Bai] },
        Jiho => { text: "ji'o", selmaho: [Bai] },
        Jihohe => { text: "ji'o'e", selmaho: [Ui, Ui3a] },
        Jihoho => { text: "ji'o'o", selmaho: [Ui, Ui3a] },
        Jihu => { text: "ji'u", selmaho: [Bai] },
        Jo => { text: "jo", selmaho: [Ja, Jehi] },
        Joha => { text: "jo'a", selmaho: [Ui] },
        Johai => { text: "jo'ai", selmaho: [Jai] },
        Johau => { text: "jo'au", selmaho: [Joi] },
        Johe => { text: "jo'e", selmaho: [Joi] },
        Johi => { text: "jo'i", selmaho: [Johi] },
        Johiha => { text: "jo'i'a", selmaho: [Joi] },
        Joho => { text: "jo'o", selmaho: [By] },
        Johu => { text: "jo'u", selmaho: [Joi] },
        Johuhu => { text: "jo'u'u", selmaho: [Joi] },
        Joi => { text: "joi", selmaho: [Joi] },
        Joihe => { text: "joi'e", selmaho: [Joi] },
        Joihi => { text: "joi'i", selmaho: [Vuhu] },
        Joiho => { text: "joi'o", selmaho: [By] },
        Joihu => { text: "joi'u", selmaho: [By] },
        Ju => { text: "ju", selmaho: [Ja, Jehi] },
        Juha => { text: "ju'a", selmaho: [Ui] },
        Juhe => { text: "ju'e", selmaho: [Joi] },
        Juhi => { text: "ju'i", selmaho: [Coi] },
        Juho => { text: "ju'o", selmaho: [Ui] },
        Juhoi => { text: "ju'oi", selmaho: [Ui] },
        Juhu => { text: "ju'u", selmaho: [Vuhu] },
        Jy => { text: "jy", selmaho: [By] },
        Ka => { text: "ka", selmaho: [Nu] },
        Kaha => { text: "ka'a", selmaho: [Bai] },
        Kahai => { text: "ka'ai", selmaho: [Bai, Nu] },
        Kahe => { text: "ka'e", selmaho: [Caha] },
        Kahi => { text: "ka'i", selmaho: [Bai] },
        Kaho => { text: "ka'o", selmaho: [Pa] },
        Kahu => { text: "ka'u", selmaho: [Ui] },
        Kai => { text: "kai", selmaho: [Bai] },
        Kaihai => { text: "kai'ai", selmaho: [Nu] },
        Kaihu => { text: "kai'u", selmaho: [Nu] },
        Kau => { text: "kau", selmaho: [Ui] },
        Kauha => { text: "kau'a", selmaho: [By] },
        Kauhe => { text: "kau'e", selmaho: [By] },
        Kauhi => { text: "kau'i", selmaho: [By] },
        Kauho => { text: "kau'o", selmaho: [By] },
        Kauhu => { text: "kau'u", selmaho: [By] },
        Ke => { text: "ke", selmaho: [] },
        Keha => { text: "ke'a", selmaho: [Koha] },
        Kehau => { text: "ke'au", selmaho: [Zohu] },
        Kehe => { text: "ke'e", selmaho: [] },
        Kehi => { text: "ke'i", selmaho: [Gaho] },
        Kehihai => { text: "ke'i'ai", selmaho: [Ui, Ui3a] },
        Keho => { text: "ke'o", selmaho: [Coi] },
        Kehu => { text: "ke'u", selmaho: [Ui] },
        Kei => { text: "kei", selmaho: [] },
        Ki => { text: "ki", selmaho: [] },
        Kiha => { text: "ki'a", selmaho: [Ui] },
        Kihaha => { text: "ki'a'a", selmaho: [Koha] },
        Kihai => { text: "ki'ai", selmaho: [Bai, Coi, Ui, Ui3a] },
        Kihe => { text: "ki'e", selmaho: [Coi] },
        Kiheha => { text: "ki'e'a", selmaho: [Koha] },
        Kihi => { text: "ki'i", selmaho: [Bai, Nu] },
        Kihiha => { text: "ki'i'a", selmaho: [Koha] },
        Kiho => { text: "ki'o", selmaho: [Pa] },
        Kihoha => { text: "ki'o'a", selmaho: [Koha] },
        Kihohe => { text: "ki'o'e", selmaho: [Bai] },
        Kihoi => { text: "ki'oi", selmaho: [Bai] },
        Kihu => { text: "ki'u", selmaho: [Bai] },
        Kihuha => { text: "ki'u'a", selmaho: [Koha] },
        Kihuhe => { text: "ki'u'e", selmaho: [Bai] },
        Kihuhi => { text: "ki'u'i", selmaho: [Bai] },
        Ko => { text: "ko", selmaho: [Koha] },
        Koha => { text: "ko'a", selmaho: [Koha] },
        Kohau => { text: "ko'au", selmaho: [Bai] },
        Kohe => { text: "ko'e", selmaho: [Koha] },
        Kohi => { text: "ko'i", selmaho: [Koha] },
        Koho => { text: "ko'o", selmaho: [Koha] },
        Kohoi => { text: "ko'oi", selmaho: [Ui] },
        Kohu => { text: "ko'u", selmaho: [Koha] },
        Koi => { text: "koi", selmaho: [Bai] },
        Ku => { text: "ku", selmaho: [Ku] },
        Kuha => { text: "ku'a", selmaho: [Joi] },
        Kuhau => { text: "ku'au", selmaho: [] },
        Kuhe => { text: "ku'e", selmaho: [] },
        Kuhi => { text: "ku'i", selmaho: [Ui] },
        Kuho => { text: "ku'o", selmaho: [] },
        Kuhoi => { text: "ku'oi", selmaho: [] },
        Kuhu => { text: "ku'u", selmaho: [Bai] },
        Ky => { text: "ky", selmaho: [By] },
        La => { text: "la", selmaho: [La] },
        Laha => { text: "la'a", selmaho: [Ui] },
        Lahai => { text: "la'ai", selmaho: [Bai] },
        Lahau => { text: "la'au", selmaho: [Lu] },
        Lahe => { text: "la'e", selmaho: [Lahe] },
        Lahei => { text: "la'ei", selmaho: [Bai, Le, Ui, Ui3a] },
        Lahi => { text: "la'i", selmaho: [La] },
        Laho => { text: "la'o", selmaho: [Zoi] },
        Lahoho => { text: "la'o'o", selmaho: [Bai] },
        Lahoi => { text: "la'oi", selmaho: [Ui, Ui3a] },
        Lahu => { text: "la'u", selmaho: [Bai] },
        Lai => { text: "lai", selmaho: [La] },
        Lau => { text: "lau", selmaho: [Lau] },
        Le => { text: "le", selmaho: [Le] },
        Leha => { text: "le'a", selmaho: [Bai] },
        Lehai => { text: "le'ai", selmaho: [] },
        Lehe => { text: "le'e", selmaho: [Le] },
        Lehei => { text: "le'ei", selmaho: [Le] },
        Lehi => { text: "le'i", selmaho: [Le] },
        Leho => { text: "le'o", selmaho: [Ui] },
        Lehohe => { text: "le'o'e", selmaho: [Ui, Ui3a] },
        Lehu => { text: "le'u", selmaho: [Lehu] },
        Lei => { text: "lei", selmaho: [Le] },
        Leihe => { text: "lei'e", selmaho: [Le] },
        Leihi => { text: "lei'i", selmaho: [Le] },
        Li => { text: "li", selmaho: [Li] },
        Liha => { text: "li'a", selmaho: [Ui] },
        Lihai => { text: "li'ai", selmaho: [Li] },
        Lihau => { text: "li'au", selmaho: [Lihau] },
        Lihe => { text: "li'e", selmaho: [Bai] },
        Lihehe => { text: "li'e'e", selmaho: [Bai] },
        Lihei => { text: "li'ei", selmaho: [Bai, Li] },
        Lihi => { text: "li'i", selmaho: [Nu] },
        Liho => { text: "li'o", selmaho: [Ui] },
        Lihoi => { text: "li'oi", selmaho: [Ui] },
        Lihu => { text: "li'u", selmaho: [Lihu] },
        Lo => { text: "lo", selmaho: [Le] },
        Loha => { text: "lo'a", selmaho: [By] },
        Lohai => { text: "lo'ai", selmaho: [] },
        Lohe => { text: "lo'e", selmaho: [Le] },
        Lohei => { text: "lo'ei", selmaho: [Le] },
        Lohi => { text: "lo'i", selmaho: [Le] },
        Loho => { text: "lo'o", selmaho: [Loho] },
        Lohoi => { text: "lo'oi", selmaho: [Lohoi] },
        Lohu => { text: "lo'u", selmaho: [Lohu] },
        Loi => { text: "loi", selmaho: [Le] },
        Loihe => { text: "loi'e", selmaho: [Lahe, Le] },
        Loihi => { text: "loi'i", selmaho: [Lahe, Le] },
        Lu => { text: "lu", selmaho: [Lu] },
        Luha => { text: "lu'a", selmaho: [Lahe] },
        Luhe => { text: "lu'e", selmaho: [Lahe] },
        Luhei => { text: "lu'ei", selmaho: [Luhei] },
        Luhi => { text: "lu'i", selmaho: [Lahe] },
        Luho => { text: "lu'o", selmaho: [Lahe] },
        Luhu => { text: "lu'u", selmaho: [] },
        Ly => { text: "ly", selmaho: [By] },
        Ma => { text: "ma", selmaho: [Koha] },
        Maha => { text: "ma'a", selmaho: [Koha] },
        Mahai => { text: "ma'ai", selmaho: [Ui, Ui3a] },
        Mahau => { text: "ma'au", selmaho: [Koha] },
        Mahe => { text: "ma'e", selmaho: [Bai] },
        Mahei => { text: "ma'ei", selmaho: [Bai, Koha] },
        Mahi => { text: "ma'i", selmaho: [Bai] },
        Maho => { text: "ma'o", selmaho: [] },
        Mahoi => { text: "ma'oi", selmaho: [Koha, Zo] },
        Mahu => { text: "ma'u", selmaho: [Pa] },
        Mai => { text: "mai", selmaho: [Mai] },
        Maiho => { text: "mai'o", selmaho: [Li] },
        Mau => { text: "mau", selmaho: [Bai] },
        Mauha => { text: "mau'a", selmaho: [Lohoi] },
        Mauhe => { text: "mau'e", selmaho: [To] },
        Mauhi => { text: "mau'i", selmaho: [Bai] },
        Mauho => { text: "mau'o", selmaho: [Toi] },
        Mauhu => { text: "mau'u", selmaho: [Bai] },
        Me => { text: "me", selmaho: [] },
        Meha => { text: "me'a", selmaho: [Bai] },
        Mehau => { text: "me'au", selmaho: [Me] },
        Mehe => { text: "me'e", selmaho: [Bai] },
        Mehei => { text: "me'ei", selmaho: [Le, Pa] },
        Mehi => { text: "me'i", selmaho: [Pa] },
        Meho => { text: "me'o", selmaho: [Li] },
        Mehohe => { text: "me'o'e", selmaho: [Lahe] },
        Mehoi => { text: "me'oi", selmaho: [] },
        Mehu => { text: "me'u", selmaho: [] },
        Mei => { text: "mei", selmaho: [Moi] },
        Mi => { text: "mi", selmaho: [Koha] },
        Miha => { text: "mi'a", selmaho: [Koha] },
        Mihai => { text: "mi'ai", selmaho: [Koha] },
        Mihau => { text: "mi'au", selmaho: [Koha] },
        Mihe => { text: "mi'e", selmaho: [Coi] },
        Mihei => { text: "mi'ei", selmaho: [Coi] },
        Mihi => { text: "mi'i", selmaho: [Bihi] },
        Miho => { text: "mi'o", selmaho: [Koha] },
        Mihu => { text: "mi'u", selmaho: [Ui] },
        Mo => { text: "mo", selmaho: [Goha] },
        Moha => { text: "mo'a", selmaho: [Pa] },
        Mohe => { text: "mo'e", selmaho: [] },
        Mohi => { text: "mo'i", selmaho: [Mohi] },
        Moho => { text: "mo'o", selmaho: [Koha, Mai] },
        Mohoi => { text: "mo'oi", selmaho: [Le] },
        Mohu => { text: "mo'u", selmaho: [Koha, Zaho] },
        Moi => { text: "moi", selmaho: [Moi] },
        Moiho => { text: "moi'o", selmaho: [Moi] },
        Moihoi => { text: "moi'oi", selmaho: [Le] },
        Mu => { text: "mu", selmaho: [Pa] },
        Muha => { text: "mu'a", selmaho: [Ui] },
        Muhai => { text: "mu'ai", selmaho: [Bai] },
        Muhe => { text: "mu'e", selmaho: [Nu] },
        Muhei => { text: "mu'ei", selmaho: [Bai, Roi, Ui, Ui3a] },
        Muhi => { text: "mu'i", selmaho: [Bai] },
        Muho => { text: "mu'o", selmaho: [Coi] },
        Muhoi => { text: "mu'oi", selmaho: [Bai, Zoi] },
        Muhu => { text: "mu'u", selmaho: [Bai] },
        My => { text: "my", selmaho: [By] },
        Na => { text: "na", selmaho: [Na] },
        Naha => { text: "na'a", selmaho: [By] },
        Nahe => { text: "na'e", selmaho: [Nahe] },
        Nahei => { text: "na'ei", selmaho: [Nahe] },
        Nahi => { text: "na'i", selmaho: [Ui] },
        Naho => { text: "na'o", selmaho: [Tahe] },
        Nahoi => { text: "na'oi", selmaho: [Se] },
        Nahu => { text: "na'u", selmaho: [] },
        Nai => { text: "nai", selmaho: [Nai] },
        Nau => { text: "nau", selmaho: [Cuhe] },
        Nauho => { text: "nau'o", selmaho: [Koha] },
        Nauhu => { text: "nau'u", selmaho: [Koha] },
        Ne => { text: "ne", selmaho: [Goi] },
        Neha => { text: "ne'a", selmaho: [Faha] },
        Nehahi => { text: "ne'a'i", selmaho: [Bai] },
        Nehi => { text: "ne'i", selmaho: [Faha] },
        Neho => { text: "ne'o", selmaho: [Vuhu] },
        Nehu => { text: "ne'u", selmaho: [Faha] },
        Nei => { text: "nei", selmaho: [Goha] },
        Ni => { text: "ni", selmaho: [Nu] },
        Niha => { text: "ni'a", selmaho: [Faha] },
        Nihe => { text: "ni'e", selmaho: [] },
        Nihei => { text: "ni'ei", selmaho: [Ui, Ui3a] },
        Nihi => { text: "ni'i", selmaho: [Bai] },
        Nihihi => { text: "ni'i'i", selmaho: [Bai] },
        Niho => { text: "ni'o", selmaho: [Niho] },
        Nihu => { text: "ni'u", selmaho: [Pa] },
        No => { text: "no", selmaho: [Pa] },
        Noha => { text: "no'a", selmaho: [Goha] },
        Nohe => { text: "no'e", selmaho: [Nahe] },
        Nohei => { text: "no'ei", selmaho: [Nahe] },
        Nohi => { text: "no'i", selmaho: [Niho] },
        Noho => { text: "no'o", selmaho: [Pa] },
        Nohoi => { text: "no'oi", selmaho: [Noi, Ui, Ui3a] },
        Nohu => { text: "no'u", selmaho: [Goi] },
        Noi => { text: "noi", selmaho: [Noi] },
        Noiha => { text: "noi'a", selmaho: [Noiha] },
        Noihi => { text: "noi'i", selmaho: [To] },
        Noihoha => { text: "noi'o'a", selmaho: [Noiha] },
        Nu => { text: "nu", selmaho: [Nu] },
        Nuha => { text: "nu'a", selmaho: [] },
        Nuhe => { text: "nu'e", selmaho: [Coi] },
        Nuhi => { text: "nu'i", selmaho: [] },
        Nuho => { text: "nu'o", selmaho: [Caha] },
        Nuhu => { text: "nu'u", selmaho: [] },
        Ny => { text: "ny", selmaho: [By] },
        O => { text: "o", selmaho: [A, By] },
        Oha => { text: "o'a", selmaho: [Ui] },
        Ohai => { text: "o'ai", selmaho: [Coi] },
        Ohe => { text: "o'e", selmaho: [Ui] },
        Ohi => { text: "o'i", selmaho: [Ui] },
        Oho => { text: "o'o", selmaho: [Ui] },
        Ohu => { text: "o'u", selmaho: [Ui] },
        Ohy => { text: "o'y", selmaho: [By] },
        Oi => { text: "oi", selmaho: [Ui] },
        Oiha => { text: "oi'a", selmaho: [Ui] },
        Oihoi => { text: "oi'oi", selmaho: [Ui, Ui3a] },
        Pa => { text: "pa", selmaho: [Pa] },
        Paha => { text: "pa'a", selmaho: [Bai] },
        Pahahi => { text: "pa'a'i", selmaho: [Bai] },
        Pahe => { text: "pa'e", selmaho: [Ui] },
        Pahi => { text: "pa'i", selmaho: [Vuhu] },
        Paho => { text: "pa'o", selmaho: [Faha] },
        Pahu => { text: "pa'u", selmaho: [Bai] },
        Pai => { text: "pai", selmaho: [Pa] },
        Paihe => { text: "pai'e", selmaho: [Nu] },
        Pau => { text: "pau", selmaho: [Ui] },
        Pe => { text: "pe", selmaho: [Goi] },
        Peha => { text: "pe'a", selmaho: [Ui] },
        Pehahi => { text: "pe'a'i", selmaho: [Bai] },
        Pehe => { text: "pe'e", selmaho: [Pehe] },
        Pehei => { text: "pe'ei", selmaho: [Coi] },
        Pehi => { text: "pe'i", selmaho: [Ui] },
        Peho => { text: "pe'o", selmaho: [] },
        Pehu => { text: "pe'u", selmaho: [Coi] },
        Pei => { text: "pei", selmaho: [Cai] },
        Peihe => { text: "pei'e", selmaho: [Coi] },
        Pi => { text: "pi", selmaho: [Pa] },
        Piha => { text: "pi'a", selmaho: [Vuhu] },
        Pihai => { text: "pi'ai", selmaho: [Vuhu] },
        Pihe => { text: "pi'e", selmaho: [Pa] },
        Pihei => { text: "pi'ei", selmaho: [Lahe] },
        Pihi => { text: "pi'i", selmaho: [Vuhu] },
        Piho => { text: "pi'o", selmaho: [Bai] },
        Pihu => { text: "pi'u", selmaho: [Joi] },
        Po => { text: "po", selmaho: [Goi] },
        Pohai => { text: "po'ai", selmaho: [Ui, Ui3a] },
        Pohe => { text: "po'e", selmaho: [Goi] },
        Pohi => { text: "po'i", selmaho: [Bai] },
        Poho => { text: "po'o", selmaho: [Ui] },
        Pohoi => { text: "po'oi", selmaho: [Lahe, Noi] },
        Pohu => { text: "po'u", selmaho: [Goi] },
        Poi => { text: "poi", selmaho: [Noi] },
        Poiha => { text: "poi'a", selmaho: [Noiha] },
        Poihei => { text: "poi'ei", selmaho: [Lahe] },
        Poihi => { text: "poi'i", selmaho: [Nu] },
        Poihoha => { text: "poi'o'a", selmaho: [Noiha] },
        Pu => { text: "pu", selmaho: [Pu] },
        Puha => { text: "pu'a", selmaho: [Bai] },
        Puhau => { text: "pu'au", selmaho: [Cuhe] },
        Puhe => { text: "pu'e", selmaho: [Bai] },
        Puhehi => { text: "pu'e'i", selmaho: [Bai] },
        Puhi => { text: "pu'i", selmaho: [Caha] },
        Puhiha => { text: "pu'i'a", selmaho: [Bai] },
        Puhihi => { text: "pu'i'i", selmaho: [Bai] },
        Puho => { text: "pu'o", selmaho: [Zaho] },
        Puhohi => { text: "pu'o'i", selmaho: [Bai] },
        Puhu => { text: "pu'u", selmaho: [Nu] },
        Py => { text: "py", selmaho: [By] },
        Ra => { text: "ra", selmaho: [Koha] },
        Raha => { text: "ra'a", selmaho: [Bai] },
        Rahai => { text: "ra'ai", selmaho: [Koha] },
        Rahe => { text: "ra'e", selmaho: [Pa] },
        Rahi => { text: "ra'i", selmaho: [Bai] },
        Raho => { text: "ra'o", selmaho: [] },
        Rahoi => { text: "ra'oi", selmaho: [] },
        Rahu => { text: "ra'u", selmaho: [Ui] },
        Rai => { text: "rai", selmaho: [Bai] },
        Raihe => { text: "rai'e", selmaho: [Bai] },
        Rau => { text: "rau", selmaho: [Pa] },
        Rauhi => { text: "rau'i", selmaho: [Koha] },
        Re => { text: "re", selmaho: [Pa] },
        Reha => { text: "re'a", selmaho: [Vuhu] },
        Rehe => { text: "re'e", selmaho: [Ui] },
        Rehei => { text: "re'ei", selmaho: [Coi] },
        Rehi => { text: "re'i", selmaho: [Coi] },
        Reho => { text: "re'o", selmaho: [Faha] },
        Rehu => { text: "re'u", selmaho: [Roi] },
        Rei => { text: "rei", selmaho: [Pa] },
        Ri => { text: "ri", selmaho: [Koha] },
        Riha => { text: "ri'a", selmaho: [Bai] },
        Rihe => { text: "ri'e", selmaho: [Ui] },
        Rihi => { text: "ri'i", selmaho: [Bai] },
        Rihiha => { text: "ri'i'a", selmaho: [Bai] },
        Rihihe => { text: "ri'i'e", selmaho: [Bai] },
        Rihihi => { text: "ri'i'i", selmaho: [Bai] },
        Rihiho => { text: "ri'i'o", selmaho: [Bai] },
        Rihihu => { text: "ri'i'u", selmaho: [Bai] },
        Riho => { text: "ri'o", selmaho: [Vuhu] },
        Rihoi => { text: "ri'oi", selmaho: [Le] },
        Rihu => { text: "ri'u", selmaho: [Faha] },
        Ro => { text: "ro", selmaho: [Pa] },
        Roha => { text: "ro'a", selmaho: [Ui] },
        Rohe => { text: "ro'e", selmaho: [Ui] },
        Rohei => { text: "ro'ei", selmaho: [Koha] },
        Rohi => { text: "ro'i", selmaho: [Ui] },
        Roho => { text: "ro'o", selmaho: [Ui] },
        Rohoi => { text: "ro'oi", selmaho: [Pa] },
        Rohu => { text: "ro'u", selmaho: [Ui] },
        Roi => { text: "roi", selmaho: [Roi] },
        Ru => { text: "ru", selmaho: [Koha] },
        Ruha => { text: "ru'a", selmaho: [Ui] },
        Ruhe => { text: "ru'e", selmaho: [Cai] },
        Ruhi => { text: "ru'i", selmaho: [Tahe] },
        Ruho => { text: "ru'o", selmaho: [By] },
        Ruhu => { text: "ru'u", selmaho: [Faha] },
        Ry => { text: "ry", selmaho: [By] },
        Sa => { text: "sa", selmaho: [Sa] },
        Saha => { text: "sa'a", selmaho: [Ui] },
        Sahai => { text: "sa'ai", selmaho: [] },
        Sahe => { text: "sa'e", selmaho: [Ui] },
        Sahei => { text: "sa'ei", selmaho: [Coi] },
        Sahi => { text: "sa'i", selmaho: [Vuhu] },
        Sahiha => { text: "sa'i'a", selmaho: [Vuhu] },
        Saho => { text: "sa'o", selmaho: [Vuhu] },
        Sahu => { text: "sa'u", selmaho: [Ui] },
        Sai => { text: "sai", selmaho: [Cai] },
        Saihe => { text: "sai'e", selmaho: [Sei] },
        Saihi => { text: "sai'i", selmaho: [Ui, Ui3a] },
        Sau => { text: "sau", selmaho: [Bai] },
        Sauha => { text: "sau'a", selmaho: [Zaho] },
        Se => { text: "se", selmaho: [Se] },
        Seha => { text: "se'a", selmaho: [Ui] },
        Sehe => { text: "se'e", selmaho: [Koha] },
        Sehi => { text: "se'i", selmaho: [Ui] },
        Seho => { text: "se'o", selmaho: [Ui] },
        Sehu => { text: "se'u", selmaho: [] },
        Sei => { text: "sei", selmaho: [Sei] },
        Seiha => { text: "sei'a", selmaho: [Ui, Ui3a] },
        Seihe => { text: "sei'e", selmaho: [Sei] },
        Seihi => { text: "sei'i", selmaho: [Ui, Ui3a] },
        Si => { text: "si", selmaho: [Si] },
        Siha => { text: "si'a", selmaho: [Ui] },
        Sihau => { text: "si'au", selmaho: [Ui] },
        Sihe => { text: "si'e", selmaho: [Moi] },
        Sihi => { text: "si'i", selmaho: [Vuhu] },
        Siho => { text: "si'o", selmaho: [Nu] },
        Sihu => { text: "si'u", selmaho: [Bai] },
        So => { text: "so", selmaho: [Pa] },
        Soha => { text: "so'a", selmaho: [Pa] },
        Sohahu => { text: "so'a'u", selmaho: [Ui, Ui3a] },
        Sohai => { text: "so'ai", selmaho: [Koha, Pa] },
        Sohe => { text: "so'e", selmaho: [Pa] },
        Sohei => { text: "so'ei", selmaho: [Pa, Ui, Ui3a] },
        Sohi => { text: "so'i", selmaho: [Pa] },
        Soho => { text: "so'o", selmaho: [Pa] },
        Sohoi => { text: "so'oi", selmaho: [Pa] },
        Sohu => { text: "so'u", selmaho: [Pa] },
        Soi => { text: "soi", selmaho: [Soi] },
        Soiha => { text: "soi'a", selmaho: [Noiha] },
        Soihe => { text: "soi'e", selmaho: [Sei] },
        Su => { text: "su", selmaho: [Su] },
        Suha => { text: "su'a", selmaho: [Ui] },
        Suhai => { text: "su'ai", selmaho: [Nu, Pa] },
        Suhe => { text: "su'e", selmaho: [Pa] },
        Suhei => { text: "su'ei", selmaho: [Se, Ui, Ui3a] },
        Suhi => { text: "su'i", selmaho: [Vuhu] },
        Suho => { text: "su'o", selmaho: [Pa] },
        Suhoi => { text: "su'oi", selmaho: [Pa, Sei] },
        Suhu => { text: "su'u", selmaho: [Nu] },
        Sy => { text: "sy", selmaho: [By] },
        Ta => { text: "ta", selmaho: [Koha] },
        Taha => { text: "ta'a", selmaho: [Coi] },
        Tahai => { text: "ta'ai", selmaho: [] },
        Tahe => { text: "ta'e", selmaho: [Tahe] },
        Tahi => { text: "ta'i", selmaho: [Bai] },
        Tahiha => { text: "ta'i'a", selmaho: [Bai] },
        Tahihe => { text: "ta'i'e", selmaho: [Bai] },
        Tahihi => { text: "ta'i'i", selmaho: [Bai] },
        Tahiho => { text: "ta'i'o", selmaho: [Bai] },
        Tahihu => { text: "ta'i'u", selmaho: [Bai] },
        Taho => { text: "ta'o", selmaho: [Ui] },
        Tahu => { text: "ta'u", selmaho: [Ui] },
        Tahuhi => { text: "ta'u'i", selmaho: [Bai] },
        Tai => { text: "tai", selmaho: [Bai] },
        Tau => { text: "tau", selmaho: [Lau] },
        Te => { text: "te", selmaho: [Se] },
        Teha => { text: "te'a", selmaho: [Vuhu] },
        Tehai => { text: "te'ai", selmaho: [Bai, Xi] },
        Tehe => { text: "te'e", selmaho: [Faha] },
        Teho => { text: "te'o", selmaho: [Pa] },
        Tehoi => { text: "te'oi", selmaho: [Lahe] },
        Tehu => { text: "te'u", selmaho: [] },
        Tei => { text: "tei", selmaho: [] },
        Ti => { text: "ti", selmaho: [Koha] },
        Tiha => { text: "ti'a", selmaho: [Faha] },
        Tihau => { text: "ti'au", selmaho: [Koha] },
        Tihe => { text: "ti'e", selmaho: [Ui] },
        Tihi => { text: "ti'i", selmaho: [Bai] },
        Tihiha => { text: "ti'i'a", selmaho: [Bai] },
        Tiho => { text: "ti'o", selmaho: [Sei] },
        Tihu => { text: "ti'u", selmaho: [Bai] },
        Tihuha => { text: "ti'u'a", selmaho: [Bai] },
        Tihuhi => { text: "ti'u'i", selmaho: [Bai] },
        Tihuhu => { text: "ti'u'u", selmaho: [Bai] },
        To => { text: "to", selmaho: [To] },
        Toha => { text: "to'a", selmaho: [By] },
        Tohai => { text: "to'ai", selmaho: [Se] },
        Tohe => { text: "to'e", selmaho: [Nahe] },
        Tohi => { text: "to'i", selmaho: [To] },
        Toho => { text: "to'o", selmaho: [Faha] },
        Tohohe => { text: "to'o'e", selmaho: [Koha] },
        Tohu => { text: "to'u", selmaho: [Ui] },
        Toi => { text: "toi", selmaho: [Toi] },
        Tu => { text: "tu", selmaho: [Koha] },
        Tuha => { text: "tu'a", selmaho: [Lahe] },
        Tuhai => { text: "tu'ai", selmaho: [Lu] },
        Tuhau => { text: "tu'au", selmaho: [Koha] },
        Tuhe => { text: "tu'e", selmaho: [Tuhe] },
        Tuhi => { text: "tu'i", selmaho: [Bai] },
        Tuhiha => { text: "tu'i'a", selmaho: [Bai] },
        Tuhihe => { text: "tu'i'e", selmaho: [Bai] },
        Tuhihi => { text: "tu'i'i", selmaho: [Bai] },
        Tuhiho => { text: "tu'i'o", selmaho: [Bai] },
        Tuhihu => { text: "tu'i'u", selmaho: [Bai] },
        Tuho => { text: "tu'o", selmaho: [Pa] },
        Tuhu => { text: "tu'u", selmaho: [] },
        Ty => { text: "ty", selmaho: [By] },
        U => { text: "u", selmaho: [A, By] },
        Ua => { text: "ua", selmaho: [Ui] },
        Ue => { text: "ue", selmaho: [Ui] },
        Uehi => { text: "ue'i", selmaho: [Ui] },
        Uha => { text: "u'a", selmaho: [Ui] },
        Uhe => { text: "u'e", selmaho: [Ui] },
        Uhi => { text: "u'i", selmaho: [Ui] },
        Uho => { text: "u'o", selmaho: [Ui] },
        Uhohe => { text: "u'o'e", selmaho: [Ui, Ui3a] },
        Uhohi => { text: "u'o'i", selmaho: [Ui, Ui3a] },
        Uhoho => { text: "u'o'o", selmaho: [Ui, Ui3a] },
        Uhohu => { text: "u'o'u", selmaho: [Ui, Ui3a] },
        Uhoi => { text: "u'oi", selmaho: [Ui, Ui3a] },
        Uhu => { text: "u'u", selmaho: [Ui] },
        Uhy => { text: "u'y", selmaho: [By] },
        Ui => { text: "ui", selmaho: [Ui] },
        Uihai => { text: "ui'ai", selmaho: [Ui, Ui3a] },
        Uo => { text: "uo", selmaho: [Ui] },
        Uu => { text: "uu", selmaho: [Ui] },
        Uy => { text: "uy", selmaho: [By] },
        Va => { text: "va", selmaho: [Va] },
        Vaha => { text: "va'a", selmaho: [Vuhu] },
        Vahe => { text: "va'e", selmaho: [Moi] },
        Vahei => { text: "va'ei", selmaho: [Roi] },
        Vahi => { text: "va'i", selmaho: [Ui] },
        Vaho => { text: "va'o", selmaho: [Bai] },
        Vahohi => { text: "va'o'i", selmaho: [Bai] },
        Vahu => { text: "va'u", selmaho: [Bai] },
        Vai => { text: "vai", selmaho: [Pa] },
        Vaihe => { text: "vai'e", selmaho: [Ui, Ui3a] },
        Vau => { text: "vau", selmaho: [Vau] },
        Ve => { text: "ve", selmaho: [Se] },
        Veha => { text: "ve'a", selmaho: [Veha] },
        Vehe => { text: "ve'e", selmaho: [Veha] },
        Vehi => { text: "ve'i", selmaho: [Veha] },
        Veho => { text: "ve'o", selmaho: [Veho] },
        Vehu => { text: "ve'u", selmaho: [Veha] },
        Vei => { text: "vei", selmaho: [Vei] },
        Vi => { text: "vi", selmaho: [Va] },
        Viha => { text: "vi'a", selmaho: [Viha] },
        Vihe => { text: "vi'e", selmaho: [Viha] },
        Vihi => { text: "vi'i", selmaho: [Viha] },
        Viho => { text: "vi'o", selmaho: [Coi] },
        Vihu => { text: "vi'u", selmaho: [Viha] },
        Vo => { text: "vo", selmaho: [Pa] },
        Voha => { text: "vo'a", selmaho: [Koha] },
        Vohai => { text: "vo'ai", selmaho: [Se] },
        Vohe => { text: "vo'e", selmaho: [Koha] },
        Vohi => { text: "vo'i", selmaho: [Koha] },
        Voho => { text: "vo'o", selmaho: [Koha] },
        Vohu => { text: "vo'u", selmaho: [Koha] },
        Voi => { text: "voi", selmaho: [Noi] },
        Voihe => { text: "voi'e", selmaho: [Goi, Lahe] },
        Voihi => { text: "voi'i", selmaho: [Noi] },
        Vu => { text: "vu", selmaho: [Va] },
        Vuha => { text: "vu'a", selmaho: [Faha] },
        Vuhe => { text: "vu'e", selmaho: [Ui] },
        Vuhi => { text: "vu'i", selmaho: [Lahe] },
        Vuho => { text: "vu'o", selmaho: [] },
        Vuhu => { text: "vu'u", selmaho: [Vuhu] },
        Vy => { text: "vy", selmaho: [By] },
        Xa => { text: "xa", selmaho: [Pa] },
        Xaho => { text: "xa'o", selmaho: [Zaho] },
        Xai => { text: "xai", selmaho: [Koha] },
        Xaihe => { text: "xai'e", selmaho: [Pa] },
        Xauha => { text: "xau'a", selmaho: [Lohoi, Ui, Ui3a] },
        Xauhe => { text: "xau'e", selmaho: [Pa, Ui, Ui3a] },
        Xauhi => { text: "xau'i", selmaho: [Ui, Ui3a] },
        Xauho => { text: "xau'o", selmaho: [Ui, Ui3a] },
        Xauhu => { text: "xau'u", selmaho: [Ui, Ui3a] },
        Xe => { text: "xe", selmaho: [Se] },
        Xehau => { text: "xe'au", selmaho: [Sehu] },
        Xehe => { text: "xe'e", selmaho: [Pa] },
        Xehei => { text: "xe'ei", selmaho: [Nu] },
        Xehiha => { text: "xe'i'a", selmaho: [Ui, Ui3a] },
        Xehihe => { text: "xe'i'e", selmaho: [Ui, Ui3a] },
        Xehihi => { text: "xe'i'i", selmaho: [Ui, Ui3a] },
        Xehiho => { text: "xe'i'o", selmaho: [Ui, Ui3a] },
        Xehihu => { text: "xe'i'u", selmaho: [Ui, Ui3a] },
        Xehu => { text: "xe'u", selmaho: [Goha] },
        Xeihe => { text: "xei'e", selmaho: [Faha] },
        Xi => { text: "xi", selmaho: [Xi] },
        Xihe => { text: "xi'e", selmaho: [Xi] },
        Xihi => { text: "xi'i", selmaho: [Xi] },
        Xo => { text: "xo", selmaho: [Pa] },
        Xohai => { text: "xo'ai", selmaho: [Pa, Se] },
        Xohe => { text: "xo'e", selmaho: [Pa] },
        Xohi => { text: "xo'i", selmaho: [Me] },
        Xoho => { text: "xo'o", selmaho: [Ui] },
        Xohu => { text: "xo'u", selmaho: [Pa, Zaho] },
        Xoi => { text: "xoi", selmaho: [Soi] },
        Xoihi => { text: "xoi'i", selmaho: [Pa] },
        Xu => { text: "xu", selmaho: [Ui] },
        Xuhai => { text: "xu'ai", selmaho: [Bai] },
        Xuhau => { text: "xu'au", selmaho: [Roi] },
        Xuhei => { text: "xu'ei", selmaho: [Coi] },
        Xuhu => { text: "xu'u", selmaho: [Lohoi] },
        Xy => { text: "xy", selmaho: [By] },
        Y => { text: "y", selmaho: [Y] },
        Yhy => { text: "y'y", selmaho: [By] },
        Za => { text: "za", selmaho: [Zi] },
        Zaha => { text: "za'a", selmaho: [Ui] },
        Zahai => { text: "za'ai", selmaho: [Nu, Pa] },
        Zahe => { text: "za'e", selmaho: [Bahe] },
        Zahei => { text: "za'ei", selmaho: [Ui, Ui3a] },
        Zahi => { text: "za'i", selmaho: [Nu] },
        Zaho => { text: "za'o", selmaho: [Zaho] },
        Zahoha => { text: "za'o'a", selmaho: [Ui, Ui3a] },
        Zahu => { text: "za'u", selmaho: [Pa] },
        Zai => { text: "zai", selmaho: [Lau] },
        Zau => { text: "zau", selmaho: [Bai] },
        Zauha => { text: "zau'a", selmaho: [Bai] },
        Zauhe => { text: "zau'e", selmaho: [Bai] },
        Zauhi => { text: "zau'i", selmaho: [Bai] },
        Zauho => { text: "zau'o", selmaho: [Bai] },
        Zauhu => { text: "zau'u", selmaho: [Bai] },
        Ze => { text: "ze", selmaho: [Pa] },
        Zeha => { text: "ze'a", selmaho: [Zeha] },
        Zehe => { text: "ze'e", selmaho: [Zeha] },
        Zehi => { text: "ze'i", selmaho: [Zeha] },
        Zeho => { text: "ze'o", selmaho: [Faha] },
        Zehoi => { text: "ze'oi", selmaho: [] },
        Zehu => { text: "ze'u", selmaho: [Zeha] },
        Zei => { text: "zei", selmaho: [Zei] },
        Zi => { text: "zi", selmaho: [Zi] },
        Zihe => { text: "zi'e", selmaho: [] },
        Ziho => { text: "zi'o", selmaho: [Koha] },
        Zo => { text: "zo", selmaho: [Zo] },
        Zoha => { text: "zo'a", selmaho: [Faha] },
        Zohau => { text: "zo'au", selmaho: [Le] },
        Zohe => { text: "zo'e", selmaho: [Koha] },
        Zohei => { text: "zo'ei", selmaho: [Koha, Lahe] },
        Zohi => { text: "zo'i", selmaho: [Faha] },
        Zoho => { text: "zo'o", selmaho: [Ui] },
        Zohoi => { text: "zo'oi", selmaho: [Ui, Ui3a] },
        Zohu => { text: "zo'u", selmaho: [Zohu] },
        Zoi => { text: "zoi", selmaho: [Zoi] },
        Zu => { text: "zu", selmaho: [Zi] },
        Zuha => { text: "zu'a", selmaho: [Faha] },
        Zuhai => { text: "zu'ai", selmaho: [Bai, Koha] },
        Zuhau => { text: "zu'au", selmaho: [Faha] },
        Zuhe => { text: "zu'e", selmaho: [Bai] },
        Zuhi => { text: "zu'i", selmaho: [Koha] },
        Zuho => { text: "zu'o", selmaho: [Nu] },
        Zuhu => { text: "zu'u", selmaho: [Ui] },
        Zy => { text: "zy", selmaho: [By] },
            ]
        }
    };
}

macro_rules! declare_cmavo_enum {
    (($($arg:expr),*) [$($variant:ident => { text: $text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Cmavo {
            $($variant,)+
        }
    };
}

macro_rules! cmavo_all {
    (() [$($variant:ident => { text: $text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        &[$(Self::$variant,)+]
    };
}

macro_rules! cmavo_from_text {
    (($text:expr) [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        match $text {
            $($canonical_text => Self::$variant,)+
            _ => return None,
        }
    };
}

macro_rules! cmavo_canonical_text {
    (($cmavo:expr) [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        match $cmavo {
            $(Self::$variant => $canonical_text,)+
        }
    };
}

macro_rules! cmavo_variant_name {
    (($cmavo:expr) [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        match $cmavo {
            $(Self::$variant => stringify!($variant),)+
        }
    };
}

macro_rules! cmavo_variant_names {
    (() [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        &[$(stringify!($variant),)+]
    };
}

macro_rules! cmavo_canonical_texts {
    (() [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($selmaho:ident),* $(,)?] }),+ $(,)?]) => {
        &[$($canonical_text,)+]
    };
}

macro_rules! cmavo_selmaho_contains {
    (($selmaho:expr, $cmavo:expr) [$($variant:ident => { text: $canonical_text:literal, selmaho: [$($member:ident),* $(,)?] }),+ $(,)?]) => {
        match $cmavo {
            $(Cmavo::$variant => false $(|| matches!($selmaho, Self::$member))* ,)+
        }
    };
}

cmavo_table!(declare_cmavo_enum);

const CMAVO_VARIANT_NAMES: &[&str] = cmavo_table!(cmavo_variant_names);
const CMAVO_CANONICAL_TEXTS: &[&str] = cmavo_table!(cmavo_canonical_texts);

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteOpenerKind {
    QuotedWords,
    DelimitedNonLojban,
    ParsedWord,
    SingleWord,
}

impl Cmavo {
    /// Every known cmavo variant, in enum declaration order.
    pub const ALL: &'static [Self] = cmavo_table!(cmavo_all);

    #[requires(true)]
    #[ensures(ret.is_some() -> !text.is_empty())]
    pub fn from_text(text: &str) -> Option<Self> {
        Some(cmavo_table!(
            cmavo_from_text,
            crate::canonicalize_text(text).as_str()
        ))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn canonical_text(self) -> &'static str {
        cmavo_table!(cmavo_canonical_text, self)
    }

    /// Stable Rust variant name generated from the canonical cmavo table.
    ///
    /// This is metadata for projections that need an identifier as well as
    /// the canonical Lojban spelling. Both values remain sourced from the same
    /// table as the enum declaration and parser lookup.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn variant_name(self) -> &'static str {
        cmavo_table!(cmavo_variant_name, self)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_selmaho(self, selmaho: Selmaho) -> bool {
        selmaho.contains(self)
    }

    /// Whether this cmavo opens any morphology-level quote construct.
    #[requires(true)]
    #[ensures(ret == self.quote_opener_kind().is_some())]
    pub const fn is_quote_opener(self) -> bool {
        self.quote_opener_kind().is_some()
    }

    /// Whether this cmavo quotes exactly one following source word as verbatim text.
    #[requires(true)]
    #[ensures(ret == matches!(self.quote_opener_kind(), Some(QuoteOpenerKind::SingleWord)))]
    pub const fn is_single_word_quote_opener(self) -> bool {
        matches!(self.quote_opener_kind(), Some(QuoteOpenerKind::SingleWord))
    }

    /// Whether this cmavo opens a delimiter-based non-Lojban quote.
    #[requires(true)]
    #[ensures(ret == matches!(self.quote_opener_kind(), Some(QuoteOpenerKind::DelimitedNonLojban)))]
    pub const fn is_delimited_non_lojban_quote_opener(self) -> bool {
        matches!(
            self.quote_opener_kind(),
            Some(QuoteOpenerKind::DelimitedNonLojban)
        )
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) const fn quote_opener_kind(self) -> Option<QuoteOpenerKind> {
        match self {
            Self::Lohu => Some(QuoteOpenerKind::QuotedWords),
            Self::Zoi | Self::Laho | Self::Muhoi => Some(QuoteOpenerKind::DelimitedNonLojban),
            Self::Zo | Self::Mahoi => Some(QuoteOpenerKind::ParsedWord),
            Self::Zohoi
            | Self::Lahoi
            | Self::Rahoi
            | Self::Mehoi
            | Self::Gohoi
            | Self::Zehoi
            | Self::Tahai
            | Self::Bohei => Some(QuoteOpenerKind::SingleWord),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|selmaho| selmaho.contains(self)))]
    pub fn primary_selmaho(self) -> Option<Selmaho> {
        Selmaho::ALL
            .iter()
            .copied()
            .find(|selmaho| selmaho.contains(self))
    }
}

#[requires(true)]
#[ensures(true)]
const fn static_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[requires(true)]
#[ensures(ret -> left.len() == right.len())]
const fn static_str_eq_ignore_ascii_case(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index].to_ascii_lowercase() != right[index].to_ascii_lowercase() {
            return false;
        }
        index += 1;
    }
    true
}

#[requires(true)]
#[ensures(ret)]
const fn cmavo_metadata_is_unique() -> bool {
    let mut left_index = 0;
    while left_index < CMAVO_VARIANT_NAMES.len() {
        let mut right_index = left_index + 1;
        while right_index < CMAVO_VARIANT_NAMES.len() {
            if static_str_eq_ignore_ascii_case(
                CMAVO_VARIANT_NAMES[left_index],
                CMAVO_VARIANT_NAMES[right_index],
            ) || static_str_eq(
                CMAVO_CANONICAL_TEXTS[left_index],
                CMAVO_CANONICAL_TEXTS[right_index],
            ) {
                return false;
            }
            right_index += 1;
        }
        left_index += 1;
    }
    true
}

const _: () = assert!(
    cmavo_metadata_is_unique(),
    "projected cmavo member names and canonical spellings must be unique"
);

impl fmt::Display for Cmavo {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_text())
    }
}

macro_rules! define_selmaho {
    ($( $variant:ident => $name:literal ),+ $(,)?) => {
        #[invariant(true)]
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Selmaho {
            $( $variant, )+
        }

        impl Selmaho {
            /// Load-bearing primary-selma'o precedence.
            ///
            /// `Cmavo::primary_selmaho` returns the first entry here that contains the
            /// cmavo. Keep this table complete and reorder it only as an intentional
            /// precedence change for multi-selma'o cmavo.
            pub const ALL: &'static [Self] = &[$( Self::$variant, )+];

            #[requires(true)]
            #[ensures(!ret.is_empty())]
            pub const fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )+
                }
            }

            #[requires(!name.is_empty())]
            #[ensures(ret.is_none() || ret.unwrap().name() == name)]
            pub fn from_name(name: &str) -> Option<Self> {
                Some(match name {
                    $( $name => Self::$variant, )+
                    _ => return None,
                })
            }

            #[requires(true)]
            #[ensures(true)]
            pub const fn contains(self, cmavo: Cmavo) -> bool {
                cmavo_table!(cmavo_selmaho_contains, self, cmavo)
            }
        }
    };
}

define_selmaho! {
    A => "A",
    Bahe => "BAhE",
    Bai => "BAI",
    Be => "BE",
    Beho => "BEhO",
    Bei => "BEI",
    Bihi => "BIhI",
    Bu => "BU",
    By => "BY",
    Caha => "CAhA",
    Cai => "CAI",
    Cehe => "CEhE",
    Co => "CO",
    Coi => "COI",
    Cu => "CU",
    Cuhe => "CUhE",
    Daho => "DAhO",
    Doi => "DOI",
    Fa => "FA",
    Faha => "FAhA",
    Faho => "FAhO",
    Fuha => "FUhA",
    Ga => "GA",
    Gaho => "GAhO",
    Gi => "GI",
    Giha => "GIhA",
    Gihi => "GIhI",
    Goha => "GOhA",
    Goi => "GOI",
    Guha => "GUhA",
    I => "I",
    Ja => "JA",
    Jai => "JAI",
    Jehi => "JEhI",
    Johi => "JOhI",
    Joi => "JOI",
    Koha => "KOhA",
    Ku => "KU",
    La => "LA",
    Lahe => "LAhE",
    Lau => "LAU",
    Le => "LE",
    Lehu => "LEhU",
    Li => "LI",
    Lihau => "LIhAU",
    Lihu => "LIhU",
    Loho => "LOhO",
    Lohoi => "LOhOI",
    Lohu => "LOhU",
    Lu => "LU",
    Luhei => "LUhEI",
    Mai => "MAI",
    Me => "ME",
    Mohe => "MOhE",
    Mohi => "MOhI",
    Moi => "MOI",
    Na => "NA",
    Nahe => "NAhE",
    Nai => "NAI",
    Niho => "NIhO",
    Noi => "NOI",
    Noiha => "NOIhA",
    Nu => "NU",
    Pa => "PA",
    Pehe => "PEhE",
    Pu => "PU",
    Roi => "ROI",
    Sa => "SA",
    Se => "SE",
    Sehu => "SEhU",
    Sei => "SEI",
    Si => "SI",
    Soi => "SOI",
    Su => "SU",
    Tahe => "TAhE",
    To => "TO",
    Toi => "TOI",
    Tuhe => "TUhE",
    Ui => "UI",
    Ui3a => "UI3a",
    Va => "VA",
    Vau => "VAU",
    Veha => "VEhA",
    Veho => "VEhO",
    Vei => "VEI",
    Viha => "VIhA",
    Vuhu => "VUhU",
    Xi => "XI",
    Y => "Y",
    Zaho => "ZAhO",
    Zeha => "ZEhA",
    Zei => "ZEI",
    Zi => "ZI",
    Zo => "ZO",
    Zohu => "ZOhU",
    Zoi => "ZOI",
}

#[requires(true)]
#[ensures(ret)]
const fn selmaho_metadata_is_unique() -> bool {
    let mut left_index = 0;
    while left_index < Selmaho::ALL.len() {
        let left = Selmaho::ALL[left_index];
        let mut right_index = left_index + 1;
        while right_index < Selmaho::ALL.len() {
            let right = Selmaho::ALL[right_index];
            if static_str_eq(left.name(), right.name()) {
                return false;
            }
            right_index += 1;
        }
        left_index += 1;
    }
    true
}

const _: () = assert!(
    selmaho_metadata_is_unique(),
    "projected selma'o names must be unique"
);
