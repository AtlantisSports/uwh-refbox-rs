`timescale 1ns / 1ps

typedef struct packed {
    logic a, b, c, d, e, f, g;
} digit;

typedef struct packed {
    logic fifteen, thirty, forty_five, sixty, interstice;
} t_o_time;

localparam digit OFF_DIGIT = '{a:0, b:0, c:0, d:0, e:0, f:0, g:0};
localparam digit FLASH_DIGIT = '{a:1, b:1, c:1, d:1, e:1, f:1, g:1};

localparam t_o_time FLASH_T_O_TIME = '{fifteen: 1, thirty: 1, forty_five: 1, sixty: 1, interstice: 1};

localparam int NUM_DATA_BITS = 20;

module segments (
    input [7:0] data [(NUM_DATA_BITS - 1):0],
    output digit bs_10, bs_1, ws_10, ws_1, m_10, m_1, s_10, s_1,
    output t_o_time bto, wto,
    output logic bto_ind, wto_ind, rto_ind,
    output logic fst_hlf, hlf_tm, snd_hlf, overtime, sdn_dth,
    output colon
);
    wire off;
    wire [NUM_DATA_BITS - 1:0] bit_ands;

    genvar i;
    generate
        for (i = 0; i < NUM_DATA_BITS; i = i + 1) begin
            assign bit_ands[i] = |data[i];
        end
    endgenerate

    assign off = ~(|bit_ands);

    wire flash;
    assign flash = data[0][1];

    wire [7:0] bs_bcd;
    wire bs_leading_zero;
    bin_to_bcd bs_bin_to_bcd(data[6][6:0], bs_bcd, bs_leading_zero);
    digit bs_10_values;
    data_to_digit bs_10_digit(bs_bcd[7:4], bs_10_values);
    assign bs_10 = flash ? FLASH_DIGIT : (bs_leading_zero || off) ? OFF_DIGIT : bs_10_values;
    digit bs_1_values;
    data_to_digit bs_1_digit(bs_bcd[3:0], bs_1_values);
    assign bs_1 = flash ? FLASH_DIGIT : off ? OFF_DIGIT : bs_1_values;

    wire [7:0] ws_bcd;
    wire ws_leading_zero;
    bin_to_bcd ws_bin_to_bcd(data[7][6:0], ws_bcd, ws_leading_zero);
    digit ws_10_values;
    data_to_digit ws_10_digit(ws_bcd[7:4], ws_10_values);
    assign ws_10 = flash ? FLASH_DIGIT : (ws_leading_zero || off) ? OFF_DIGIT : ws_10_values;
    digit ws_1_values;
    data_to_digit ws_1_digit(ws_bcd[3:0], ws_1_values);
    assign ws_1 = flash ? FLASH_DIGIT : off ? OFF_DIGIT : ws_1_values;

    logic [6:0] minutes;
    logic [6:0] seconds;
    shortint unsigned time_secs;
    always_comb begin
        time_secs[15:8] = data[2];
        time_secs[7:0] = data[3];
    end

    always_comb begin
        if (time_secs >= 6000) begin
            minutes = 100;  // Will get displayed as --
            seconds = 100;
        end else begin
            minutes = 7'(time_secs / 16'd60);
            seconds = 7'(time_secs % 16'd60);
        end
    end

    wire [7:0] m_bcd;
    wire m_leading_zero;
    bin_to_bcd m_bin_to_bcd(minutes, m_bcd, m_leading_zero);
    digit m_10_values;
    data_to_digit m_10_digit(m_bcd[7:4], m_10_values);
    assign m_10 = flash ? FLASH_DIGIT : (m_leading_zero || off) ? OFF_DIGIT : m_10_values;
    digit m_1_values;
    data_to_digit m_1_digit(m_bcd[3:0], m_1_values);
    assign m_1 = flash ? FLASH_DIGIT : off ? OFF_DIGIT : m_1_values;

    wire [7:0] s_bcd;
    wire s_leading_zero;
    bin_to_bcd s_bin_to_bcd(seconds, s_bcd, s_leading_zero);
    digit s_10_values;
    data_to_digit s_10_digit(s_bcd[7:4], s_10_values);
    assign s_10 = flash ? FLASH_DIGIT : off ? OFF_DIGIT : s_10_values;
    digit s_1_values;
    data_to_digit s_1_digit(s_bcd[3:0], s_1_values);
    assign s_1 = flash ? FLASH_DIGIT : off ? OFF_DIGIT : s_1_values;

    shortint unsigned timeout_secs;
    assign timeout_secs[7:0] = data[5];
    assign timeout_secs[12:8] = data[4][4:0];
    assign timeout_secs[15:13] = 3'b0;

    t_o_time timeout_val;
    assign timeout_val = '{fifteen: timeout_secs > 0, thirty: timeout_secs > 15, forty_five: timeout_secs > 30, sixty: timeout_secs > 45, interstice: 1'b0};

    logic is_bto, is_wto;
    assign is_bto = data[4][7:5] == 3'd1;
    assign is_wto = data[4][7:5] == 3'd2;

    assign bto = flash ? FLASH_T_O_TIME : '{fifteen: is_bto & timeout_val.fifteen, thirty: is_bto & timeout_val.thirty, forty_five: is_bto & timeout_val.forty_five, sixty: is_bto & timeout_val.sixty, interstice: is_bto & timeout_val.interstice};
    assign wto = flash ? FLASH_T_O_TIME : '{fifteen: is_wto & timeout_val.fifteen, thirty: is_wto & timeout_val.thirty, forty_five: is_wto & timeout_val.forty_five, sixty: is_wto & timeout_val.sixty, interstice: is_wto & timeout_val.interstice};

    assign bto_ind = is_bto | flash;
    assign wto_ind = is_wto | flash;
    assign rto_ind = data[4][7:5] == 3'd3 | data[4][7:5] == 3'd4 | flash;

    always_comb begin
        case (data[1][3:0])
            4'd0: begin  // Between Games
                fst_hlf = flash;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = flash;
                sdn_dth = flash;
            end
            4'd1: begin  // First Half
                fst_hlf = 1;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = flash;
                sdn_dth = flash;
            end
            4'd2: begin  // Half Time
                fst_hlf = 1;
                hlf_tm = 1;
                snd_hlf = flash;
                overtime = flash;
                sdn_dth = flash;
            end
            4'd3: begin  // Second Half
                fst_hlf = 1;
                hlf_tm = 1;
                snd_hlf = 1;
                overtime = flash;
                sdn_dth = flash;
            end
            4'd4: begin  // Pre Overtime
                fst_hlf = flash;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = 1;
                sdn_dth = flash;
            end
            4'd5: begin  // Overtime First Half
                fst_hlf = 1;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = 1;
                sdn_dth = flash;
            end
            4'd6: begin  // Overtime Half Time
                fst_hlf = 1;
                hlf_tm = 1;
                snd_hlf = flash;
                overtime = 1;
                sdn_dth = flash;
            end
            4'd7: begin  // Overtime Second Half
                fst_hlf = 1;
                hlf_tm = 1;
                snd_hlf = 1;
                overtime = 1;
                sdn_dth = flash;
            end
            4'd8,  // Pre Suden Death
            4'd9: begin  // Suden Death
                fst_hlf = flash;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = flash;
                sdn_dth = 1;
            end
            default: begin
                fst_hlf = flash;
                hlf_tm = flash;
                snd_hlf = flash;
                overtime = flash;
                sdn_dth = flash;
            end
        endcase
    end

    assign colon = ~off;
    
endmodule

module bin_to_bcd (
    input [6:0] bin,
    output reg [7:0] bcd,
    output reg leading_zero
);

    always @(*) begin
        case (bin) 
            7'b0000000: bcd = 8'b00000000;  // 0
            7'b0000001: bcd = 8'b00000001;  // 1
            7'b0000010: bcd = 8'b00000010;  // 2
            7'b0000011: bcd = 8'b00000011;  // 3
            7'b0000100: bcd = 8'b00000100;  // 4
            7'b0000101: bcd = 8'b00000101;  // 5
            7'b0000110: bcd = 8'b00000110;  // 6
            7'b0000111: bcd = 8'b00000111;  // 7
            7'b0001000: bcd = 8'b00001000;  // 8
            7'b0001001: bcd = 8'b00001001;  // 9
            7'b0001010: bcd = 8'b00010000;  // 10
            7'b0001011: bcd = 8'b00010001;  // 11
            7'b0001100: bcd = 8'b00010010;  // 12
            7'b0001101: bcd = 8'b00010011;  // 13
            7'b0001110: bcd = 8'b00010100;  // 14
            7'b0001111: bcd = 8'b00010101;  // 15
            7'b0010000: bcd = 8'b00010110;  // 16
            7'b0010001: bcd = 8'b00010111;  // 17
            7'b0010010: bcd = 8'b00011000;  // 18
            7'b0010011: bcd = 8'b00011001;  // 19
            7'b0010100: bcd = 8'b00100000;  // 20
            7'b0010101: bcd = 8'b00100001;  // 21
            7'b0010110: bcd = 8'b00100010;  // 22
            7'b0010111: bcd = 8'b00100011;  // 23
            7'b0011000: bcd = 8'b00100100;  // 24
            7'b0011001: bcd = 8'b00100101;  // 25
            7'b0011010: bcd = 8'b00100110;  // 26
            7'b0011011: bcd = 8'b00100111;  // 27
            7'b0011100: bcd = 8'b00101000;  // 28
            7'b0011101: bcd = 8'b00101001;  // 29
            7'b0011110: bcd = 8'b00110000;  // 30
            7'b0011111: bcd = 8'b00110001;  // 31
            7'b0100000: bcd = 8'b00110010;  // 32
            7'b0100001: bcd = 8'b00110011;  // 33
            7'b0100010: bcd = 8'b00110100;  // 34
            7'b0100011: bcd = 8'b00110101;  // 35
            7'b0100100: bcd = 8'b00110110;  // 36
            7'b0100101: bcd = 8'b00110111;  // 37
            7'b0100110: bcd = 8'b00111000;  // 38
            7'b0100111: bcd = 8'b00111001;  // 39
            7'b0101000: bcd = 8'b01000000;  // 40
            7'b0101001: bcd = 8'b01000001;  // 41
            7'b0101010: bcd = 8'b01000010;  // 42
            7'b0101011: bcd = 8'b01000011;  // 43
            7'b0101100: bcd = 8'b01000100;  // 44
            7'b0101101: bcd = 8'b01000101;  // 45
            7'b0101110: bcd = 8'b01000110;  // 46
            7'b0101111: bcd = 8'b01000111;  // 47
            7'b0110000: bcd = 8'b01001000;  // 48
            7'b0110001: bcd = 8'b01001001;  // 49
            7'b0110010: bcd = 8'b01010000;  // 50
            7'b0110011: bcd = 8'b01010001;  // 51
            7'b0110100: bcd = 8'b01010010;  // 52
            7'b0110101: bcd = 8'b01010011;  // 53
            7'b0110110: bcd = 8'b01010100;  // 54
            7'b0110111: bcd = 8'b01010101;  // 55
            7'b0111000: bcd = 8'b01010110;  // 56
            7'b0111001: bcd = 8'b01010111;  // 57
            7'b0111010: bcd = 8'b01011000;  // 58
            7'b0111011: bcd = 8'b01011001;  // 59
            7'b0111100: bcd = 8'b01100000;  // 60
            7'b0111101: bcd = 8'b01100001;  // 61
            7'b0111110: bcd = 8'b01100010;  // 62
            7'b0111111: bcd = 8'b01100011;  // 63
            7'b1000000: bcd = 8'b01100100;  // 64
            7'b1000001: bcd = 8'b01100101;  // 65
            7'b1000010: bcd = 8'b01100110;  // 66
            7'b1000011: bcd = 8'b01100111;  // 67
            7'b1000100: bcd = 8'b01101000;  // 68
            7'b1000101: bcd = 8'b01101001;  // 69
            7'b1000110: bcd = 8'b01110000;  // 70
            7'b1000111: bcd = 8'b01110001;  // 71
            7'b1001000: bcd = 8'b01110010;  // 72
            7'b1001001: bcd = 8'b01110011;  // 73
            7'b1001010: bcd = 8'b01110100;  // 74
            7'b1001011: bcd = 8'b01110101;  // 75
            7'b1001100: bcd = 8'b01110110;  // 76
            7'b1001101: bcd = 8'b01110111;  // 77
            7'b1001110: bcd = 8'b01111000;  // 78
            7'b1001111: bcd = 8'b01111001;  // 79
            7'b1010000: bcd = 8'b10000000;  // 80
            7'b1010001: bcd = 8'b10000001;  // 81
            7'b1010010: bcd = 8'b10000010;  // 82
            7'b1010011: bcd = 8'b10000011;  // 83
            7'b1010100: bcd = 8'b10000100;  // 84
            7'b1010101: bcd = 8'b10000101;  // 85
            7'b1010110: bcd = 8'b10000110;  // 86
            7'b1010111: bcd = 8'b10000111;  // 87
            7'b1011000: bcd = 8'b10001000;  // 88
            7'b1011001: bcd = 8'b10001001;  // 89
            7'b1011010: bcd = 8'b10010000;  // 90
            7'b1011011: bcd = 8'b10010001;  // 91
            7'b1011100: bcd = 8'b10010010;  // 92
            7'b1011101: bcd = 8'b10010011;  // 93
            7'b1011110: bcd = 8'b10010100;  // 94
            7'b1011111: bcd = 8'b10010101;  // 95
            7'b1100000: bcd = 8'b10010110;  // 96
            7'b1100001: bcd = 8'b10010111;  // 97
            7'b1100010: bcd = 8'b10011000;  // 98
            7'b1100011: bcd = 8'b10011001;  // 99
            default: bcd = 8'b11111111;  // --
        endcase

        case (bin)
            7'b0000000,
            7'b0000001,
            7'b0000010,
            7'b0000011,
            7'b0000100,
            7'b0000101,
            7'b0000110,
            7'b0000111,
            7'b0001000,
            7'b0001001: leading_zero = 1'b1;
            default: leading_zero = 1'b0;
        endcase
    end

endmodule

module data_to_digit (
    input [3:0] data,
    output digit seg
);

    always @(*) begin
        case (data)
            4'b0000: seg = '{a:1, b:1, c:1, d:1, e:1, f:1, g:0}; // 0
            4'b0001: seg = '{a:0, b:1, c:1, d:0, e:0, f:0, g:0}; // 1
            4'b0010: seg = '{a:1, b:1, c:0, d:1, e:1, f:0, g:1}; // 2
            4'b0011: seg = '{a:1, b:1, c:1, d:1, e:0, f:0, g:1}; // 3
            4'b0100: seg = '{a:0, b:1, c:1, d:0, e:0, f:1, g:1}; // 4
            4'b0101: seg = '{a:1, b:0, c:1, d:1, e:0, f:1, g:1}; // 5
            4'b0110: seg = '{a:1, b:0, c:1, d:1, e:1, f:1, g:1}; // 6
            4'b0111: seg = '{a:1, b:1, c:1, d:0, e:0, f:0, g:0}; // 7
            4'b1000: seg = '{a:1, b:1, c:1, d:1, e:1, f:1, g:1}; // 8
            4'b1001: seg = '{a:1, b:1, c:1, d:1, e:0, f:1, g:1}; // 9
            4'b1010,
            4'b1011,
            4'b1100,
            4'b1101,
            4'b1110,
            4'b1111: seg = '{a:0, b:0, c:0, d:0, e:0, f:0, g:1}; // -
        endcase
    end
endmodule
